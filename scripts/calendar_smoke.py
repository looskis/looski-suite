#!/usr/bin/env python3
"""Calendar, booking embed and headless CMS acceptance against a disposable fixture clone.
No real provider credentials, recipients or source fixture mutations.
"""
import contextlib, json, os, secrets, socket, subprocess, tempfile, threading, time, uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.request import Request, urlopen
from urllib.error import HTTPError
from integration_fixture import Fixture
from dev import ROOT


def free_port():
    with socket.socket() as s:
        s.bind(('127.0.0.1',0)); return s.getsockname()[1]

def key():return secrets.token_hex(16)

f=Fixture()
source_base=f.base
project=f.config['COMPOSE_PROJECT_NAME']
database='communications_'+key()[:12]
proxy=project+'-communications-proxy-'+key()[:6]
processes=[]
server=None
class Connector(BaseHTTPRequestHandler):
    unavailable=False
    spans=[]
    writes=[]
    busy_calls=0
    def log_message(self,*args):pass
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        if self.unavailable:
            self.send_response(503);self.end_headers();return
        if self.path=='/v1/calendar/busy':
            Connector.busy_calls+=1;value={'busy':self.spans}
        else:
            self.writes.append(body);value={'state':'synced'}
        self.send_response(200);self.end_headers();self.wfile.write(json.dumps(value).encode())

def docker(*args,**kwargs):
    return subprocess.run(['docker',*args],check=True,**kwargs)
def sql(statement):
    return subprocess.check_output(['docker','exec','-i',project+'-db-1','psql','-U','supabase_admin','-d',database,'-At','-v','ON_ERROR_STOP=1'],input=statement,text=True).strip()
def wait_url(url):
    for _ in range(100):
        try:
            with urlopen(url,timeout=1) as r:
                if r.status==200:return
        except (OSError,HTTPError):pass
        time.sleep(.2)
    raise AssertionError('Service did not start: '+url)

try:
    with tempfile.TemporaryDirectory(prefix='looski-communications-') as tmp:
        tmp=Path(tmp)
        docker('exec',project+'-db-1','createdb','-U','postgres',database,stdout=subprocess.DEVNULL)
        # Clone data and grants to an isolated database; no migration touches the source.
        with (tmp/'database.sql').open('wb') as dump:
            docker('exec',project+'-db-1','pg_dump','-U','postgres','--no-owner','postgres',stdout=dump)
        dump_path=tmp/'database.sql'
        dump_path.write_text(dump_path.read_text().replace('CREATE EXTENSION IF NOT EXISTS pgmq WITH SCHEMA pgmq;', "CREATE EXTENSION IF NOT EXISTS pgmq WITH SCHEMA pgmq;\nSELECT pgmq.create('looski_jobs');"))
        with (tmp/'database.sql').open('rb') as dump:
            docker('exec','-i',project+'-db-1','psql','-q','-U','supabase_admin','-d',database,'-v','ON_ERROR_STOP=1',stdin=dump,stdout=subprocess.DEVNULL)
        if sql('SELECT version FROM suite.schema_version')=='36':
            sql((ROOT/'supabase/migrations/20261102000100_communications.sql').read_text())
        if sql('SELECT version FROM suite.schema_version')=='38':
            sql((ROOT/'supabase/migrations/20261104000200_calendar_jobs.sql').read_text())
        if sql('SELECT version FROM suite.schema_version')=='39':
            sql((ROOT/'supabase/migrations/20261104000300_visitors.sql').read_text())
        assert sql('SELECT version FROM suite.schema_version')=='40'
        forward="const net=require('net');for(const [port,host,target] of [[5432,'db',5432],[9999,'auth',9999]])net.createServer(c=>{const s=net.connect(target,host);c.pipe(s);s.pipe(c);s.on('error',()=>c.destroy());c.on('error',()=>s.destroy())}).listen(port,'0.0.0.0')"
        docker('run','--rm','-d','--name',proxy,'--network',project+'_default','-p','127.0.0.1::5432','-p','127.0.0.1::9999','--entrypoint','node',project+'-web:latest','-e',forward,stdout=subprocess.DEVNULL)
        ports=json.loads(subprocess.check_output(['docker','inspect',proxy,'--format','{{json .NetworkSettings.Ports}}']))
        dbport=ports['5432/tcp'][0]['HostPort'];authport=ports['9999/tcp'][0]['HostPort']
        dburl=(f.directory/'secrets/database_url').read_text().strip().replace('@db:5432/postgres','@127.0.0.1:'+dbport+'/'+database)
        (tmp/'database_url').write_text(dburl);(tmp/'database_url').chmod(0o600)
        server=ThreadingHTTPServer(('127.0.0.1',0),Connector);threading.Thread(target=server.serve_forever,daemon=True).start()
        endpoint='http://127.0.0.1:'+str(server.server_port)
        import base64
        subject=json.loads(base64.urlsafe_b64decode(f.token.split('.')[1]+'=='))['sub']
        member=sql("SELECT m.id FROM suite.memberships m JOIN suite.auth_identities a ON a.user_id=m.user_id WHERE m.workspace_id='"+f.workspace+"' AND a.subject='"+subject+"'")
        config_file=tmp/'calendars.json';config_file.write_text(json.dumps([{'workspace_id':f.workspace,'host_id':member,'endpoint':endpoint,'api_token':'x'*40}]))
        issuer=sql("SELECT issuer FROM suite.auth_identities LIMIT 1")
        port=free_port();f.base='http://localhost:'+str(port)
        environment={**os.environ,'LOOSKI_DATABASE_URL_FILE':str(tmp/'database_url'),'LOOSKI_JWT_SECRET_FILE':str(f.directory/'secrets/jwt_secret'),'LOOSKI_INVITATION_KEY_FILE':str(f.directory/'invitation-key'),'LOOSKI_AUTH_ISSUER':issuer,'LOOSKI_AUTH_INTERNAL_URL':'http://127.0.0.1:'+authport,'LOOSKI_PUBLIC_URL':source_base,'LOOSKI_AUTH_MAIL_ENABLED':'false','LOOSKI_DIAGNOSTICS_MODE':'disabled','LOOSKI_CALENDARS_CONFIG_FILE':str(config_file)}
        api_log=(tmp/'api.log').open('w+')
        processes.append(subprocess.Popen([str(ROOT/'target/debug/looski'),'api','--bind','127.0.0.1:'+str(port)],env=environment,stdout=api_log,stderr=api_log))
        wait_url(f.base+'/health/ready')
        import datetime
        root='/api/v1/workspaces/'+f.workspace
        def module(name):
            m=next(x for x in f.call(root+'/modules') if x['key']==name)
            if not m['enabled']:f.call(root+'/modules/'+name,'PATCH',{'enabled':True,'expected_version':m['version']},key=key())
        module('scheduling');module('cms')
        settings=dict(name='Calendar smoke',timezone='UTC',duration_minutes=30,buffer_before=15,buffer_after=15,
                      weekly=[dict(weekday=i,start=0,end=1440) for i in range(1,8)],accepting=True,reminder_minutes=0)
        t=f.call(root+'/scheduling/types','POST',{'settings':settings},key=key())
        assert t['host_id']==member
        day=(datetime.datetime.now(datetime.timezone.utc)+datetime.timedelta(days=20)).date().isoformat()
        at=lambda time:day+'T'+time+':00Z'
        endpoint='/api/v1/public/scheduling/types/'+t['public_key']
        Connector.spans=[dict(start=at('12:00'),end=at('13:00'))]
        slots=f.call(endpoint+'/slots?date='+day,authenticated=False)
        starts=[x['starts_at'] for x in slots]
        assert at('12:00') not in starts and at('11:30') not in starts and at('14:00') in starts
        assert Connector.busy_calls==1,'One external query per day, not per slot'
        code=secrets.token_hex(32)
        payload=dict(starts_at=at('12:00'),attendee_name='Calendar test',attendee_email='calendar@example.invalid')
        f.call(endpoint+'/book','POST',payload,expected=409,authenticated=False,headers={'X-Booking-Key':code})
        Connector.unavailable=True
        f.call(endpoint+'/slots?date='+day,expected=503,authenticated=False)
        f.call(endpoint+'/book','POST',dict(payload,starts_at=at('14:00')),expected=503,authenticated=False,headers={'X-Booking-Key':code})
        Connector.unavailable=False
        receipt=f.call(endpoint+'/book','POST',dict(payload,starts_at=at('14:00')),authenticated=False,headers={'X-Booking-Key':code})
        booking_id=receipt['id']
        assert sql("SELECT count(*) FROM scheduling.calendar_jobs WHERE appointment_id='"+booking_id+"'")=='1'
        worker_log=(tmp/'worker.log').open('w+')
        processes.append(subprocess.Popen([str(ROOT/'target/debug/looski'),'worker'],env=environment,stdout=worker_log,stderr=worker_log))
        def wait_write(version):
            for _ in range(100):
                if any(w['id']==booking_id and w['version']==version for w in Connector.writes):return
                time.sleep(.2)
            worker_log.seek(0);raise AssertionError(worker_log.read()[-2000:])
        wait_write(1)
        p='/api/v1/public/scheduling/appointments/'+booking_id
        f.call(p,'POST',{'expected_version':1,'action':'reschedule','starts_at':at('15:00')},authenticated=False,headers={'X-Booking-Key':code},key=key())
        wait_write(2)
        f.call(p,'POST',{'expected_version':2,'action':'cancel'},authenticated=False,headers={'X-Booking-Key':code},key=key())
        wait_write(3)
        assert next(w for w in Connector.writes if w['id']==booking_id and w['version']==3)['state']=='cancelled'
        print('PASS: external conflicts/buffers, one query/day, outage fails closed, durable booking/reschedule/cancel jobs')
        ct=f.call(root+'/cms/types','POST',dict(name='Headless smoke',headless=True,fields=[dict(key='body',label='Body',kind='text')]),expected=201,key=key())
        public='/api/v1/public/cms/'+ct['public_key']
        entry=f.call(root+'/cms/entries','POST',dict(type_id=ct['id'],title='Published',slug='stable-url',values={'body':'original'}),expected=201,key=key())
        ep=root+'/cms/entries/'+entry['id']
        assert f.call(public+'/data',authenticated=False)['items']==[]
        entry=f.call(ep+'/publish','POST',{'expected_version':entry['version']},key=key())
        published=f.call(public+'/data',authenticated=False)
        assert published['items'][0]['values']=={'body':'original'}
        assert f.call(public,authenticated=False)['headless']
        assert f.call(public+'/entries/stable-url',authenticated=False)['headless']
        f.call(ep,'PATCH',dict(expected_version=entry['version'],title='Secret draft',slug='draft-url',values={'body':'private draft'}),key=key())
        assert f.call(public+'/data',authenticated=False)==published
        f.call(public+'/data?after=%00',expected=400,authenticated=False)
        ct=f.call(root+'/cms/types/'+ct['id'],'PATCH',dict(name=ct['name'],fields=ct['fields'],expected_version=ct['version']),key=key())
        assert ct['headless'],'Omitted configuration update preserves headless mode'
        print('PASS: headless delivery excludes drafts, freezes published revisions/slugs, and preserves headless setting')
        webport=free_port();webbase='http://127.0.0.1:'+str(webport)
        webenv={**environment,'HOST':'127.0.0.1','PORT':str(webport),'ORIGIN':webbase,'LOOSKI_API_INTERNAL_URL':f.base}
        web_log=(tmp/'web.log').open('w+')
        processes.append(subprocess.Popen(['node',str(ROOT/'apps/web/build/index.js')],env=webenv,stdout=web_log,stderr=web_log))
        wait_url(webbase+'/health/live')
        from urllib.parse import urlencode
        import re
        def html(path,body=None):
            req=Request(webbase+path,data=None if body is None else urlencode(body).encode(),headers={'Origin':webbase,'Accept':'text/html'})
            try:response=urlopen(req,timeout=20)
            except HTTPError as error:response=error
            with response:return response.status,response.read().decode(),response.headers
        embed='/book/'+t['public_key']+'/embed?date='+day
        status,body,headers=html(embed)
        assert status==200 and headers.get('X-Frame-Options') is None and 'frame-ancestors *' in headers['Content-Security-Policy']
        code=re.search(r'name="code"[^>]*value="([a-f0-9]{64})"',body).group(1)
        status,body,headers=html(embed,dict(code=code,starts_at=at('17:00'),attendee_name='Embedded visitor',attendee_email='embed@example.invalid'))
        assert status==200 and 'target="_blank"' in body and code in body
        assert html('/book/'+t['public_key']+'?date='+day)[2]['X-Frame-Options']=='DENY'
        assert html('/c/'+ct['public_key'])[0]==404
        assert html('/c/'+ct['public_key']+'/stable-url')[0]==404
        print('PASS: cookie-free embed booking, management code/new tab, normal-page frame guard and headless HTML 404')

finally:
    for p in reversed(processes):
        p.terminate()
        with contextlib.suppress(subprocess.TimeoutExpired):p.wait(timeout=10)
        if p.poll() is None:p.kill();p.wait()
    if server:server.shutdown()
    subprocess.run(['docker','rm','-f',proxy],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    subprocess.run(['docker','exec',project+'-db-1','dropdb','-U','postgres','--force',database],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

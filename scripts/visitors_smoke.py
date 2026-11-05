#!/usr/bin/env python3
"""Visitor attribution acceptance against a disposable fixture clone.
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
database='visitors_'+key()[:12]
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
        module('visitors')
        api=root+'/visitors'
        settings=dict(name='Fixture website',origins=['https://example.invalid'],fingerprint_enabled=True,similarity_enabled=True,threshold=.95,retention_days=30,enabled=True)
        site=f.call(api+'/sites','POST',settings,key=key())
        pub='/api/v1/public/visitors/'+site['public_key']
        headers={'Origin':'https://example.invalid'}
        f.call(pub,authenticated=False,expected=404,headers={'Origin':'https://wrong.invalid'})
        assert f.call(pub,authenticated=False,headers=headers)['fingerprint_enabled']
        signals=dict(platform='Mac',browser='Chrome',major='140',gpu='gpu1',canvas='canvas1',audio='audio1',screen='1920x1080')
        token=secrets.token_hex(32)
        payload=dict(token=token,consent=True,event_id=str(uuid.uuid4()),page='/pricing?email=private@example.invalid#secret',components=signals)
        f.call(pub+'/events','POST',dict(payload,consent=False),expected=400,authenticated=False,headers=headers)
        assert f.call(api)['items']==[]
        for _ in range(2):f.call(pub+'/events','POST',payload,authenticated=False,headers=headers)
        v=f.call(api)['items'][0];assert v['views']==1
        detail=f.call(api+'/'+v['id']);assert detail['events'][0]['page']=='/pricing' and not detail['candidates']
        token2=secrets.token_hex(32)
        f.call(pub+'/events','POST',dict(payload,token=token2,event_id=str(uuid.uuid4())),authenticated=False,headers=headers)
        page=f.call(api);assert len(page['items'])==2,'Matching must not merge visitor/session identity'
        v2=next(x for x in page['items'] if x['id']!=v['id'])
        d2=f.call(api+'/'+v2['id']);assert d2['candidates'][0]['candidate_id']==v['id'] and not v2['contact_id']
        candidate=d2['candidates'][0]
        review=dict(candidate_id=v['id'],state='accepted',expected_version=candidate['version'])
        replay=key()
        f.call(api+'/'+v2['id']+'/matches','POST',review,key=replay)
        f.call(api+'/'+v2['id']+'/matches','POST',review,key=replay)
        f.call(api+'/'+v2['id']+'/matches','POST',dict(review,state='rejected'),expected=409,key=key())
        assert not f.call(api+'/'+v2['id'])['visitor']['contact_id']
        contact=sql("SELECT id FROM contacts.contacts WHERE workspace_id='"+f.workspace+"' LIMIT 1")
        assert contact
        resolved=f.call(api+'/resolve','POST',dict(site_key=site['public_key'],token=token,contact_id=contact,evidence='declared_contact'),key=key())
        assert resolved['contact_id']==contact
        assert not f.call(api+'/'+v2['id'])['visitor']['contact_id'],'No inherited contact from probabilistic evidence'
        f.call(api+'/'+v2['id']+'/contact','PUT',dict(contact_id=contact,expected_version=v2['version']),key=key())
        assert f.call(api+'/'+v2['id'])['visitor']['evidence']=='staff_review'
        assert f.cli(['visitors','get',v['id']])['visitor']['id']==v['id']
        assert not f.mcp('visitors_get',dict(workspace_id=f.workspace,id=v['id'])).get('isError',False)
        # More than one page, ordered UUID ties, no duplicate or missing export records.
        sql("INSERT INTO visitors.events(id,workspace_id,site_id,visitor_id,page) SELECT gen_random_uuid(),'"+f.workspace+"','"+site['id']+"','"+v['id']+"','/fixture' FROM generate_series(1,205)")
        events=f.call(api+'/'+v['id']+'/events');assert len(events['items'])==200 and events['next_cursor']
        rest=f.call(api+'/'+v['id']+'/events?after='+events['next_cursor']);assert len(rest['items'])==6 and rest['next_cursor'] is None
        assert len({x['id'] for x in events['items']+rest['items']})==206
        f.call('/api/v1/workspaces/'+str(uuid.uuid4())+'/visitors/'+v['id'],expected=403)
        # Remove read permission from the fixture owner's role in this disposable clone.
        role=sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
        sql("DELETE FROM suite.role_permissions WHERE workspace_id='"+f.workspace+"' AND role_id='"+role+"' AND permission_key='contacts.read'")
        f.call(api,expected=403)
        f.call(api+'/'+v['id'],expected=403)
        sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','contacts.read')")
        # Disabling matching purges evidence and prevents further collection of signals.
        site=f.call(api+'/sites','POST',dict(settings,id=site['id'],expected_version=site['version'],fingerprint_enabled=False,similarity_enabled=False),key=key())
        assert f.call(api+'/'+v['id'])['components']=={} and f.call(api+'/'+v2['id'])['candidates']==[]
        f.call(pub+'/events','POST',dict(payload,event_id=str(uuid.uuid4())),authenticated=False,headers=headers)
        assert f.call(api+'/'+v['id'])['components']=={}
        m=next(x for x in f.call(root+'/modules') if x['key']=='visitors')
        f.call(root+'/modules/visitors','PATCH',dict(enabled=False,expected_version=m['version']),key=key())
        f.call(pub+'/events','POST',dict(payload,event_id=str(uuid.uuid4())),authenticated=False,headers=headers,expected=404)
        f.call(pub+'/withdraw','POST',dict(token=token),authenticated=False,headers=headers)
        assert sql("SELECT count(*) FROM visitors.events WHERE visitor_id='"+v['id']+"'")=='0'
        module('visitors')
        f.call(api+'/'+v['id'],expected=404)
        f.call(pub+'/events','POST',dict(payload,event_id=str(uuid.uuid4())),authenticated=False,headers=headers,expected=400)
        # Worker enforces both event retention for active visitors and inactive visitor expiry.
        sql("UPDATE visitors.events SET created_at=now()-interval '60 days' WHERE visitor_id='"+v2['id']+"'")
        worker_log=(tmp/'worker.log').open('w+')
        processes.append(subprocess.Popen([str(ROOT/'target/debug/looski'),'worker'],env=environment,stdout=worker_log,stderr=worker_log))
        for _ in range(200):
            if sql("SELECT count(*) FROM visitors.events WHERE visitor_id='"+v2['id']+"'")=='0':break
            time.sleep(.2)
        else:raise AssertionError('Event retention worker did not prune old visits')
        deletekey=key()
        for _ in range(2):f.call(api+'/'+v2['id'],'DELETE',key=deletekey)
        assert f.call(api)['items']==[]
        print('PASS: consent/origin checks, event deduplication, uncertain suggestions without merges, review concurrency, explicit contact evidence, export pagination, CLI/MCP, permissions, disable purge, withdrawal while disabled and deletion')

finally:
    for p in reversed(processes):
        p.terminate()
        with contextlib.suppress(subprocess.TimeoutExpired):p.wait(timeout=10)
        if p.poll() is None:p.kill();p.wait()
    if server:server.shutdown()
    subprocess.run(['docker','rm','-f',proxy],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    subprocess.run(['docker','exec',project+'-db-1','dropdb','-U','postgres','--force',database],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

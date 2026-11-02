#!/usr/bin/env python3
"""Communications acceptance against a disposable clone of the dedicated fixture.

Starts native API/worker plus fake Blueski and HTTP-v1 email connectors. No real
provider credentials or recipients are used. The source fixture’s business data and schema are unchanged.
"""
import contextlib, hashlib, hmac, json, os, secrets, socket, subprocess, tempfile, threading, time, uuid
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
secret='communications-fixture-webhook-secret-123456'
journals={'blueski':[],'email':[]}
sends={}
lock=threading.Lock()
class Connector(BaseHTTPRequestHandler):
    blueski_version="0.2.0"
    def log_message(self,*args):pass
    def do_GET(self):
        from urllib.parse import urlsplit,parse_qs
        parts=urlsplit(self.path);after=int(parse_qs(parts.query).get('since',['0'])[0]);provider='email' if parts.path.startswith('/v1/') else 'blueski'
        if parts.path in ['/healthz','/v1/capabilities']:
            health={'installation_id':provider+'-fixture','product':'blueski','version':self.blueski_version if provider=='blueski' else 1,'idempotent_send':True}
            self.send_response(200);self.end_headers();self.wfile.write(json.dumps(health).encode());return
        with lock:events=[e for e in journals[provider] if e['id']>after][:50]
        self.send_response(200);self.end_headers();self.wfile.write(json.dumps(events).encode())
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        ref=body['client_ref']
        with lock:
            if ref in sends:assert sends[ref][0]==body
            else:sends[ref]=(body,str(uuid.uuid4()))
            mid=sends[ref][1]
        self.send_response(202);self.end_headers();self.wfile.write(json.dumps({'message_id':mid,'idempotent':True,'status':'queued'}).encode())

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
def event(n,**changes):
    return {'installation_id':'blueski-fixture','id':n,'event':'message.received','message_id':'apple-'+str(n),'provider_message_id':'apple-'+str(n),'chat_id':'iMessage;-;fixture-chat','thread_kind':'direct','membership_complete':True,'classification_conflict':False,'text':'Customer question '+str(n),'handle':'+15555550100','destination_caller_id':'support@example.invalid','protocol':'imessage','timestamp':'2026-11-02T12:00:00Z','created_at':'2026-11-02T12:00:00Z',**changes}
def webhook(account,e,expected=204,signature=None):
    body=json.dumps(e).encode();signature=signature or hmac.new(secret.encode(),body,hashlib.sha256).hexdigest()
    request=Request(f.base+'/api/v1/connectors/communications/'+account+'/events',data=body,headers={'Content-Type':'application/json','X-Blueski-Signature':signature})
    try:r=urlopen(request,timeout=10)
    except HTTPError as error:r=error
    with r:assert r.status==expected,(r.status,r.read())
def member_token(member):
    # Reuse the fixture owner's real first-party token while changing only suite
    # membership grants in this disposable database for scope/denial assertions.
    sql("DELETE FROM suite.membership_roles WHERE workspace_id='"+f.workspace+"' AND membership_id='"+member+"'")

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
        assert sql('SELECT version FROM suite.schema_version')=='37'
        forward="const net=require('net');for(const [port,host,target] of [[5432,'db',5432],[9999,'auth',9999]])net.createServer(c=>{const s=net.connect(target,host);c.pipe(s);s.pipe(c);s.on('error',()=>c.destroy());c.on('error',()=>s.destroy())}).listen(port,'0.0.0.0')"
        docker('run','--rm','-d','--name',proxy,'--network',project+'_default','-p','127.0.0.1::5432','-p','127.0.0.1::9999','--entrypoint','node',project+'-web:latest','-e',forward,stdout=subprocess.DEVNULL)
        ports=json.loads(subprocess.check_output(['docker','inspect',proxy,'--format','{{json .NetworkSettings.Ports}}']))
        dbport=ports['5432/tcp'][0]['HostPort'];authport=ports['9999/tcp'][0]['HostPort']
        dburl=(f.directory/'secrets/database_url').read_text().strip().replace('@db:5432/postgres','@127.0.0.1:'+dbport+'/'+database)
        (tmp/'database_url').write_text(dburl);(tmp/'database_url').chmod(0o600)
        server=ThreadingHTTPServer(('127.0.0.1',0),Connector);threading.Thread(target=server.serve_forever,daemon=True).start()
        endpoint='http://127.0.0.1:'+str(server.server_port)
        configs=[{'key':'fixture-'+kind,'workspace_id':f.workspace,'provider':'blueski' if kind=='blueski' else 'http_v1','installation_id':kind+'-fixture','endpoint':endpoint,'api_token':'fixture','webhook_secret':secret,'capabilities':{'channels':['email'],'send_text':True,'subjects':True,'delivery_receipts':True,'read_receipts':False,'attachments':False,'calls':False},'destinations':['support@example.invalid']} for kind in ['blueski','email']]
        config_file=tmp/'communications.json';config_file.write_text(json.dumps(configs));config_file.chmod(0o600)
        issuer=sql("SELECT issuer FROM suite.auth_identities LIMIT 1")
        port=free_port();f.base='http://localhost:'+str(port)
        environment={**os.environ,'LOOSKI_DATABASE_URL_FILE':str(tmp/'database_url'),'LOOSKI_JWT_SECRET_FILE':str(f.directory/'secrets/jwt_secret'),'LOOSKI_INVITATION_KEY_FILE':str(f.directory/'invitation-key'),'LOOSKI_AUTH_ISSUER':issuer,'LOOSKI_AUTH_INTERNAL_URL':'http://127.0.0.1:'+authport,'LOOSKI_PUBLIC_URL':source_base,'LOOSKI_AUTH_MAIL_ENABLED':'false','LOOSKI_DIAGNOSTICS_MODE':'disabled','LOOSKI_COMMUNICATIONS_CONFIG_FILE':str(config_file)}
        api_log=(tmp/'api.log').open('w+')
        processes.append(subprocess.Popen([str(ROOT/'target/debug/looski'),'api','--bind','127.0.0.1:'+str(port)],env=environment,stdout=api_log,stderr=api_log))
        wait_url(f.base+'/health/ready')
        root='/api/v1/workspaces/'+f.workspace;com=root+'/communications'
        def call(path,method='GET',body=None,expected=200,retry=None):return f.call(com+path,method,body,expected=expected,key=retry or (key() if method!='GET' else None))
        def module(enabled):
            m=next(m for m in f.call(root+'/modules') if m['key']=='communications')
            f.call(root+'/modules/communications','PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
        # Normalize the cloned module state: earlier acceptance suites may enable it.
        module(False);call('/accounts',expected=403);module(True)
        ac=call('/accounts','POST',{'name':'Blueski fixture','connector_key':'fixture-blueski'})
        assert not ac['shared'] and ac['capabilities']['send_text'] and not ac['capabilities']['attachments']
        assert secret not in json.dumps(ac) and endpoint not in json.dumps(ac)
        call('/accounts','POST',{'name':'Duplicate','connector_key':'fixture-blueski'},expected=409)
        call('/accounts','POST',{'name':'Unavailable','connector_key':'not-provisioned'},expected=404)
        # Invalid signatures, identities, and destinations cannot leak messages.
        webhook(ac['id'],event(1),403,signature='0'*64)
        webhook(ac['id'],event(1,installation_id='wrong'),400)
        webhook(ac['id'],event(1,destination_caller_id='personal@example.invalid'))
        assert not call('/conversations')['items']
        e1=event(1);webhook(ac['id'],e1);webhook(ac['id'],e1)
        webhook(ac['id'],event(1,text='conflicting replay'),409)
        c=call('/conversations')['items'][0];cid=c['id'];path='/conversations/'+cid
        assert len(call(path+'/messages')['items'])==1
        # Semantic dedup across distinct journal entries, including polling replay.
        e2=event(2,message_id='apple-1',provider_message_id='apple-1');webhook(ac['id'],e2)
        assert len(call(path+'/messages')['items'])==1
        # Receive newer webhook first; polling must still import the gap.
        e3=event(3);e4=event(4);webhook(ac['id'],e4)
        with lock:journals['blueski']=[e1,e2,e3,e4]
        sync=call('/accounts/'+ac['id']+'/sync','POST');assert sync['cursor']==4
        assert len(call(path+'/messages')['items'])==3
        page=call(path+'/messages?limit=1');assert page['next_cursor']
        assert len(call(path+'/messages?after='+page['next_cursor'])['items'])==2
        # Explicit contact links; no automatic contact creation from receiving traffic.
        contact=f.call(root+'/contacts','POST',{'display_name':'Communications customer'},expected=201,key=key())
        c=call(path,'PATCH',{'expected_version':c['version'],'contact_id':contact['id'],'archived':False})
        assert call('/conversations?contact_id='+contact['id'])['items'][0]['id']==cid
        call(path,'PATCH',{'expected_version':1,'contact_id':None,'archived':False},expected=409)
        # Independently authorized timeline and business export use the same projections.
        crm=next(m for m in f.call(root+'/modules') if m['key']=='crm')
        if not crm['enabled']:f.call(root+'/modules/crm','PATCH',{'enabled':True,'expected_version':crm['version']},key=key())
        timeline=root+'/crm/timeline?anchor_kind=contact&anchor_id='+contact['id']+'&limit=1'
        history=f.call(timeline);assert history['items'][0]['event']['kind']=='communication'
        assert f.call(timeline+'&after='+history['next_cursor'])['items']
        exported=f.call(root+'/exports/page','POST',{'collection':'communication_messages','parent_id':cid})
        assert len(exported['items'])==3 and secret not in json.dumps(exported)
        # Queue once under a stable key, then deliver with the worker.
        retry=key();body={'body':'Reply from the fixture'}
        from concurrent.futures import ThreadPoolExecutor
        with ThreadPoolExecutor(max_workers=2) as pool:
            replies=list(pool.map(lambda _:call(path+'/messages','POST',body,retry=retry),range(2)))
        assert replies[0]['id']==replies[1]['id'];m=replies[0]
        assert call(path+'/messages','POST',body,retry=retry)['id']==m['id']
        call(path+'/messages','POST',{'body':'Different'},409,retry)
        def work_until(job,state='succeeded'):
            for _ in range(30):
                subprocess.run([str(ROOT/'target/debug/looski'),'worker','--once'],env=environment,check=True,stdout=subprocess.DEVNULL,stderr=api_log)
                actual=sql("SELECT state FROM suite.jobs WHERE id='"+job+"'")
                if actual in [state,'completed'] and state=='succeeded':return
                if actual==state:return
            raise AssertionError(('job did not reach expected state',actual,state))
        work_until(m['job_id'])
        assert sends[m['id']][0]['chat_id']==c['provider_thread_id']
        assert call(path+'/messages')['items'][-1]['state']=='accepted'
        webhook(ac['id'],event(6,event='message.status',client_ref=m['id'],provider_message_id='outbound-guid',status='delivered'))
        webhook(ac['id'],event(5,event='message.queued',client_ref=m['id']))
        assert call(path+'/messages')['items'][-1]['state']=='delivered'
        # Restart/re-execute after provider acceptance: identical key, one external effect.
        sql("UPDATE communications.messages SET state='queued' WHERE id='"+m['id']+"'")
        # Force a simulated lost DB commit by requeueing the completed job via SQL in the fixture only.
        sql("UPDATE suite.jobs SET state='blocked',version=version+1 WHERE id='"+m['job_id']+"'")
        job=f.call(root+'/jobs/'+m['job_id']);f.call(root+'/jobs/'+m['job_id'],'PATCH',{'expected_version':job['version'],'action':'resume'},key=key())
        work_until(m['job_id']);assert len(sends)==1
        # Generic HTTP v1 email proves the common core is not tied to Apple threading.
        email=call('/accounts','POST',{'name':'Email fixture','connector_key':'fixture-email'})
        ee={'version':1,'installation_id':'email-fixture','id':1,'kind':'message.received','message_id':'<fixture@example.invalid>','thread_id':'email-thread-1','channel':'email','sender':'customer@example.invalid','destination':'support@example.invalid','body':'Email question','subject':'Order #100','client_ref':None,'state':'received','metadata':{'in_reply_to':'<original@example.invalid>'}}
        with lock:journals['email']=[ee]
        call('/accounts/'+email['id']+'/sync','POST');ec=next(x for x in call('/conversations')['items'] if x['channel']=='email')
        em=call('/conversations/'+ec['id']+'/messages','POST',{'body':'Email reply'});work_until(em['job_id'])
        assert sends[em['id']][0]['subject']=='Order #100' and sends[em['id']][0]['version']==1
        # CLI/MCP use the same HTTP authority and projections.
        assert len(f.cli(['communications','list'])['items'])==2
        assert not f.mcp('communications_messages',{'workspace_id':f.workspace,'id':cid}).get('isError',False)
        # Module disable blocks queued sends and stops ingestion; reenable does not auto-resume.
        pending=call(path+'/messages','POST',{'body':'Must remain blocked'})
        module(False);call('/conversations',expected=403);webhook(ac['id'],event(7),403)
        assert sql("SELECT state FROM suite.jobs WHERE id='"+pending['job_id']+"'")=='blocked'
        module(True);assert sql("SELECT state FROM suite.jobs WHERE id='"+pending['job_id']+"'")=='blocked'
        # Private account remains invisible even with workspace-wide communications grants.
        owner=sql("SELECT owner_membership_id FROM communications.accounts WHERE id='"+ac['id']+"'")
        alternate=sql("INSERT INTO suite.users(display_name) VALUES('Other owner') RETURNING id").splitlines()[0]
        alternate_member=sql("INSERT INTO suite.memberships(workspace_id,user_id) VALUES('"+f.workspace+"','"+alternate+"') RETURNING id").splitlines()[0]
        sql("UPDATE communications.accounts SET owner_membership_id='"+alternate_member+"' WHERE id='"+ac['id']+"'")
        call(path,expected=404);call(path+'/messages','POST',body,404,retry)
        assert len(call('/accounts'))==1
        assert not f.call(timeline)['items']
        f.call(root+'/exports/page','POST',{'collection':'communication_messages','parent_id':cid},expected=404)
        sql("UPDATE communications.accounts SET shared=true WHERE id='"+ac['id']+"'")
        assert call(path)['id']==cid
        # An account's private messages cannot be reached through a different workspace.
        other_ws=sql("INSERT INTO suite.workspaces(name) VALUES('Other communications workspace') RETURNING id").splitlines()[0]
        f.call('/api/v1/workspaces/'+other_ws+'/communications/conversations/'+cid,expected=404)
        sql("UPDATE communications.accounts SET owner_membership_id='"+owner+"',shared=false WHERE id='"+ac['id']+"'")
        # Lost authority is checked again at worker execution.
        deny=call(path+'/messages','POST',{'body':'Do not send after permission removal'})
        roles=sql("SELECT role_id FROM suite.membership_roles WHERE workspace_id='"+f.workspace+"' AND membership_id='"+owner+"'").splitlines()
        member_token(owner);work_until(deny['job_id'],'blocked');assert deny['id'] not in sends
        for role in roles:sql("INSERT INTO suite.membership_roles(workspace_id,membership_id,role_id) VALUES('"+f.workspace+"','"+owner+"','"+role+"')")
        # Refuse an old provider before any send, rather than discovering missing retry safety afterwards.
        Connector.blueski_version='0.1.0'
        incompatible=call(path+'/messages','POST',{'body':'Old provider must not receive this'})
        work_until(incompatible['job_id'],'blocked');assert incompatible['id'] not in sends
        Connector.blueski_version='0.2.0'
        sql("UPDATE suite.workspaces SET state='suspended' WHERE id='"+f.workspace+"'")
        webhook(ac['id'],event(8),403)
        sql("UPDATE suite.workspaces SET state='active' WHERE id='"+f.workspace+"'")
        print('PASS: migrations, HMAC, destination filtering, event/semantic dedup, replay gaps, contact links, pagination, idempotent queue/send, receipts, restart reconciliation, email adapter, CLI/MCP, privacy, workspace/module and worker authority')
        # Optional rendered browser journey through a local Svelte development server.
        if os.environ.get('COMMUNICATIONS_BROWSER')=='1':
            webport=free_port();webbase='http://localhost:'+str(webport)
            webenv={**environment,'LOOSKI_API_INTERNAL_URL':f.base}
            web_log=(tmp/'web.log').open('w+')
            processes.append(subprocess.Popen(['pnpm','--filter','@looski/web','exec','vite','--host','127.0.0.1','--port',str(webport)],cwd=ROOT,env=webenv,stdout=web_log,stderr=web_log))
            wait_url(webbase+'/health/live')
            from forms_browser_html import Forms
            from urllib.parse import urlencode
            def browser(path,body=None,expected=200):
                req=Request(webbase+path,data=None if body is None else urlencode(body).encode(),headers={'Accept':'text/html','Origin':webbase})
                try:r=f.browser.open(req,timeout=30)
                except HTTPError as e:r=e
                with r:
                    text=r.read().decode();assert r.status==expected,(r.status,text[:500]);return text
            ui='/workspaces/'+f.workspace+'/communications'
            html=browser(ui);assert 'Blueski fixture' in html and 'Order #100' in html
            html=browser(ui+'/'+cid);assert 'Customer question' in html and 'Communications customer' in html
            pairs=next(x['pairs'] for x in Forms(html).forms if x['action']=='?/send')
            pairs=[(k,v) for k,v in pairs if k!='body']+[('body','<script>browser reply</script>')]
            html=browser(ui+'/'+cid+'?/send',pairs);assert '&lt;script' in html and 'browser reply' in html and '<script>browser reply</script>' not in html
            html=browser('/workspaces/'+f.workspace+'/contacts/'+contact['id']);assert 'Conversations' in html and cid in html
            print('PASS: rendered inbox, reply form, escaped message body, linked-contact page')
finally:
    for p in reversed(processes):
        p.terminate()
        with contextlib.suppress(subprocess.TimeoutExpired):p.wait(timeout=10)
        if p.poll() is None:p.kill();p.wait()
    if server:server.shutdown()
    subprocess.run(['docker','rm','-f',proxy],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    subprocess.run(['docker','exec',project+'-db-1','dropdb','-U','postgres','--force',database],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

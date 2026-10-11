#!/usr/bin/env python3
"""Milestone 6 acceptance through real HTTP, PostgreSQL, staff sessions, CLI and MCP."""
import base64,hashlib,hmac,json,secrets,subprocess,time,uuid
from concurrent.futures import ThreadPoolExecutor
from urllib.parse import urlencode
from dev import ROOT
from integration_fixture import Fixture
f=Fixture();call,sql=f.call,f.sql
base='/api/v1/workspaces/'+f.workspace;hd=base+'/helpdesk';public='/api/v1/public/helpdesk'
def key():return secrets.token_hex(16)
def module(k,enabled):
 m=next(m for m in call(base+'/modules') if m['key']==k)
 if m['enabled']!=enabled:call(base+'/modules/'+k,'PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
def auth(path,body,token=None):
 payload={'path':path,'body':body,'token':token}
 script="const x=JSON.parse(require('fs').readFileSync(0,'utf8')); fetch('http://auth:9999'+x.path,{method:'POST',headers:{'Content-Type':'application/json',...(x.token?{Authorization:'Bearer '+x.token}:{})},body:JSON.stringify(x.body)}).then(async r=>{if(!r.ok)throw new Error(r.status+await r.text());console.log(await r.text())})"
 return json.loads(subprocess.check_output(['docker','compose','exec','-T','web','node','-e',script],input=json.dumps(payload),text=True,cwd=ROOT,env=f.environment))
def administrator():
 def b64(value):return base64.urlsafe_b64encode(value).rstrip(b'=')
 secret=(f.directory/'secrets/jwt_secret').read_bytes().strip()
 unsigned=b64(b'{"alg":"HS256","typ":"JWT"}')+b'.'+b64(json.dumps({'role':'service_role','iss':f.base+'/auth/v1','exp':int(time.time())+120}).encode())
 return (unsigned+b'.'+b64(hmac.new(secret,unsigned,hashlib.sha256).digest())).decode()
issuer=sql("SELECT i.issuer FROM suite.auth_identities i JOIN suite.memberships m ON m.user_id=i.user_id WHERE m.workspace_id='"+f.workspace+"' ORDER BY i.linked_at LIMIT 1")
class Person:
 """A real signed-in member with one role, so denials come from the running API."""
 def __init__(self,label,grants):
  email='helpdesk-'+key()+'@example.invalid';password=secrets.token_urlsafe(24)
  user=auth('/admin/users',{'email':email,'password':password,'email_confirm':True},administrator())
  self.token=auth('/token?grant_type=password',{'email':email,'password':password})['access_token']
  self.role=call(base+'/access/roles','POST',{'name':label+' '+key()[:8],'grants':self.expand(grants)},expected=201,key=key())
  self.user=sql("INSERT INTO suite.users(display_name) VALUES('"+label+"') RETURNING id").splitlines()[0]
  sql("INSERT INTO suite.auth_identities(user_id,issuer,subject) VALUES('"+self.user+"','"+issuer+"','"+user['id']+"')")
  self.membership=sql("INSERT INTO suite.memberships(workspace_id,user_id) VALUES('"+f.workspace+"','"+self.user+"') RETURNING id").splitlines()[0]
  sql("INSERT INTO suite.membership_roles(workspace_id,membership_id,role_id) VALUES('"+f.workspace+"','"+self.membership+"','"+self.role['id']+"')")
 @staticmethod
 def expand(grants):return [{'action':a.split(':')[0],'scope':(a.split(':')+['workspace'])[1]} for a in grants]
 def grants(self,grants):
  self.role=call(base+'/access/roles/'+self.role['id'],'PATCH',{'name':self.role['name'],'grants':self.expand(grants),'expected_version':self.role['version']},key=key())
 def call(self,path,method='GET',body=None,expected=200,retry=None):
  return call(path,method,body,expected=expected,key=retry or (key() if method!='GET' else None),authenticated=False,headers={'Authorization':'Bearer '+self.token})

def customer(id,code,path='',method='GET',body=None,expected=200,retry=None):
 return call(public+'/tickets/'+id+path,method,body,expected=expected,key=retry,authenticated=False,headers={'X-Ticket-Key':code})
def open_ticket(code=None,body=None):
 code=code or secrets.token_hex(32)
 payload=body or {'subject':'A support request '+key(),'requester_name':'Customer','body':'Please help with this request.'}
 receipt=call(public+'/intake/'+settings['public_key'],'POST',payload,authenticated=False,headers={'X-Ticket-Key':code})
 return receipt['id'],code,payload
def get(id):return call(hd+'/tickets/'+id)
def update(id,**changes):
 t=get(id);payload={'expected_version':t['version'],'status':t['status'],'assignee_id':t['assignee_id'],'archived':t['archived'],**changes}
 return call(hd+'/tickets/'+id,'PATCH',payload,key=key())
def reply(id,body,internal=False,retry=None):return call(hd+'/tickets/'+id+'/messages','POST',{'body':body,'internal':internal},key=retry or key())
def absent(value,*words):
 encoded=json.dumps(value)
 for word in words:assert word not in encoded,(word,encoded[:300])
initial={m['key']:m['enabled'] for m in call(base+'/modules')}
try:
 for m in initial:module(m,False)
 call(hd+'/tickets',expected=403)
 module('helpdesk',True)
 assert not any(m['enabled'] for m in call(base+'/modules') if m['key']!='helpdesk')
 settings=call(hd+'/settings');settings=call(hd+'/settings','PUT',{'accepting':True,'expected_version':settings['version']},key=key())
 intake=public+'/intake/'+settings['public_key']
 call(intake,authenticated=False)
 call(intake,'POST',{'subject':'x','requester_name':'x','body':'x'},expected=404,authenticated=False)
 secret=secrets.token_hex(32);payload={'subject':'Standalone '+key(),'requester_name':'A customer','body':'Original request'}
 # Concurrent creates and exact retry: one ticket, one original message.
 def create():return call(intake,'POST',payload,authenticated=False,headers={'X-Ticket-Key':secret})
 with ThreadPoolExecutor(max_workers=2) as pool:receipts=list(pool.map(lambda _:create(),range(2)))
 assert receipts[0]==receipts[1];tid=receipts[0]['id'];assert len(customer(tid,secret,'/messages')['items'])==1
 call(intake,'POST',{**payload,'body':'Changed'},expected=409,authenticated=False,headers={'X-Ticket-Key':secret})
 other,other_code,_=open_ticket()
 customer(tid,other_code,expected=404);customer(other,secret,expected=404)
 customer(str(uuid.uuid4()),secret,expected=404)
 call(hd+'/tickets/'+tid,authenticated=False,headers={'X-Ticket-Key':secret},expected=401)
 before=customer(tid,secret);before_messages=customer(tid,secret,'/messages')
 note_text='PRIVATE-'+key();note=reply(tid,note_text,True)
 assert customer(tid,secret)==before and customer(tid,secret,'/messages')==before_messages
 customer(tid,secret,'/messages?after='+note['id'],expected=404)
 # Only the two public message kinds can cross the customer boundary.
 answer=reply(tid,'Here is the answer.');visible=customer(tid,secret,'/messages')
 absent(visible,note_text,note['id'],'author_membership_id','customer_key_hash')
 assert [m['kind'] for m in visible['items']]==['customer','reply']
 first=customer(tid,secret,'/messages?limit=1');second=customer(tid,secret,'/messages?after='+first['next_cursor'])
 assert second['items'][0]['id']==answer['id']
 update(tid,status='resolved');retry=key()
 def send():return customer(tid,secret,'/messages','POST',{'body':'Still need help'},retry=retry)
 with ThreadPoolExecutor(max_workers=2) as pool:sent=list(pool.map(lambda _:send(),range(2)))
 assert sent[0]==sent[1] and get(tid)['status']=='open'
 customer(tid,secret,'/messages','POST',{'body':'Different'},expected=409,retry=retry)
 customer(tid,secret,'/messages','POST',{'body':'Bad','internal':True},expected=400,retry=key())
 call(hd+'/tickets/'+tid+'/messages','POST',{'body':' '},expected=400,key=key())
 # Independent staff scopes, action grants and current authority on retries.
 agent=Person('Support agent',['workspace.read','helpdesk.read:assigned','helpdesk.reply:assigned','helpdesk.manage:assigned'])
 observer=Person('Support observer',['workspace.read','helpdesk.read'])
 agent.call(hd+'/tickets/'+tid,expected=404);agent.call(hd+'/settings',expected=403)
 t=update(tid,assignee_id=agent.membership)
 assert agent.call(hd+'/tickets')['items'][0]['id']==tid
 staff_key=key();staff_body={'body':'Assigned-only private note','internal':True}
 private=agent.call(hd+'/tickets/'+tid+'/messages','POST',staff_body,retry=staff_key)
 assert agent.call(hd+'/tickets/'+tid+'/messages','POST',staff_body,retry=staff_key)==private
 observer.call(hd+'/tickets/'+tid+'/messages','POST',{'body':'No permission'},expected=403)
 update(tid,assignee_id=None)
 agent.call(hd+'/tickets/'+tid+'/messages','POST',staff_body,expected=404,retry=staff_key)
 assert agent.call(hd+'/tickets')['items']==[]
 update(tid,assignee_id=agent.membership);agent.grants(['workspace.read'])
 agent.call(hd+'/tickets/'+tid,expected=403)
 # Optimistic concurrency produces one successful update, never lost edits.
 t=get(tid);payload_update={'expected_version':t['version'],'status':'waiting','assignee_id':None,'archived':False}
 call(hd+'/tickets/'+tid,'PATCH',payload_update,key=key())
 call(hd+'/tickets/'+tid,'PATCH',payload_update,expected=409,key=key())
 update(tid,archived=True);customer(tid,secret,expected=404)
 call(hd+'/tickets/'+tid+'/messages','POST',{'body':'no'},expected=409,key=key())
 update(tid,archived=False);customer(tid,secret)
 # Public closure does not disable replies; module closure does.
 settings=call(hd+'/settings','PUT',{'accepting':False,'expected_version':settings['version']},key=key())
 call(intake,authenticated=False,expected=404);customer(tid,secret)
 module('helpdesk',False);customer(tid,secret,expected=404);call(hd+'/tickets',expected=403)
 module('helpdesk',True);assert get(tid)['id']==tid
 # Revocation closes reads, replies and their retries immediately.
 update(tid,revoke_customer_access=True)
 customer(tid,secret,expected=404);customer(tid,secret,'/messages','POST',{'body':'Still need help'},expected=404,retry=retry)
 update(tid,revoke_customer_access=False);customer(tid,secret,expected=404)
 # Knowledge links remain staff-only and re-evaluate collection authority.
 module('knowledge',True);kb=base+'/knowledge'
 shelf=call(kb+'/collections','POST',{'name':'Restricted support references','restricted':True},expected=201,key=key())
 article=call(kb+'/documents','POST',{'collection_id':shelf['id'],'title':'Secret procedure '+key(),'body':'Private procedure'},expected=201,key=key())
 contact=call(base+'/contacts','POST',{'display_name':'Support contact '+key()},expected=201,key=key())
 t=get(other);link={'expected_version':t['version'],'contact_id':contact['id'],'article_ids':[article['id']]}
 t=call(hd+'/tickets/'+other+'/links','PUT',link,key=key());assert t['articles'][0]['preview']['title']==article['title']
 hidden=observer.call(hd+'/tickets/'+other);assert hidden['contact_id'] is None and hidden['articles'][0]['preview'] is None
 absent(customer(other,other_code),article['id'],article['title'],contact['id'])
 module('knowledge',False);assert get(other)['articles'][0]['preview'] is None
 # Independent Helpdesk and Contacts authority on the CRM-composed timeline.
 module('crm',True)
 timeline=base+'/crm/timeline?'+urlencode({'anchor_kind':'contact','anchor_id':contact['id'],'limit':1})
 page=call(timeline);assert page['items'][0]['event']['kind']=='ticket'
 while page['next_cursor']:page=call(timeline+'&after='+page['next_cursor'])
 module('helpdesk',False);assert not call(timeline)['items'];module('helpdesk',True)
 # Export is the same scoped projection and never includes access code or its digest.
 exported=call(base+'/exports/page','POST',{'collection':'helpdesk_messages','parent_id':tid})
 assert any(m['body']==note_text for m in exported['items'])
 absent(exported,secret,hashlib.sha256(secret.encode()).hexdigest())
 exported=call(base+'/exports/page','POST',{'collection':'helpdesk_tickets'})
 absent(exported,secret,hashlib.sha256(secret.encode()).hexdigest(),'creation_hash')
 observer.call(base+'/exports/page','POST',{'collection':'helpdesk_tickets'},expected=403)
 # Contact merges retain typed ticket references and surface the survivor.
 survivor=call(base+'/contacts','POST',{'display_name':'Support survivor '+key()},expected=201,key=key())
 call(base+'/contacts/'+survivor['id']+'/merge','POST',{'source_id':contact['id'],'source_version':contact['version'],'survivor_version':survivor['version'],'resolved':{'display_name':survivor['display_name']}},key=key())
 assert get(other)['contact_id']==survivor['id']
 assert any(t['id']==other for t in call(hd+'/tickets?contact_id='+survivor['id'])['items'])
 assert call(base+'/crm/timeline?'+urlencode({'anchor_kind':'contact','anchor_id':survivor['id']}))['items']
 # Workspace-qualified references and scopes never cross workspaces.
 foreign=sql("INSERT INTO suite.workspaces(name) VALUES('Helpdesk foreign fixture') RETURNING id").splitlines()[0]
 foreign_member=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Foreign member') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',id FROM u RETURNING id").splitlines()[0]
 foreign_ticket=sql("INSERT INTO helpdesk.tickets(workspace_id,subject,requester_name,customer_key_hash,creation_hash) VALUES('"+foreign+"','Foreign ticket','Foreign requester',repeat('a',64),repeat('b',64)) RETURNING id").splitlines()[0]
 call('/api/v1/workspaces/'+foreign+'/helpdesk/tickets',expected=403)
 call(hd+'/tickets/'+foreign_ticket,expected=404)
 current=get(other)
 call(hd+'/tickets/'+other,'PATCH',{'expected_version':current['version'],'status':'open','archived':False,'assignee_id':foreign_member},expected=400,key=key())
 # Public required-event failure rolls back creation as well as its first message.
 settings=call(hd+'/settings');settings=call(hd+'/settings','PUT',{'accepting':True,'expected_version':settings['version']},key=key())
 sql("CREATE FUNCTION helpdesk.test_fail_event() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture event failure'; END $$; CREATE TRIGGER helpdesk_test_event BEFORE INSERT ON helpdesk.events FOR EACH ROW EXECUTE FUNCTION helpdesk.test_fail_event()")
 failed_code=secrets.token_hex(32)
 try:call(intake,'POST',{'subject':'Must not survive','requester_name':'Rollback','body':'No commit'},expected=503,authenticated=False,headers={'X-Ticket-Key':failed_code})
 finally:sql('DROP TRIGGER helpdesk_test_event ON helpdesk.events; DROP FUNCTION helpdesk.test_fail_event()')
 assert sql("SELECT count(*) FROM helpdesk.tickets WHERE customer_key_hash='"+hashlib.sha256(failed_code.encode()).hexdigest()+"'")=='0'
 # Four-interface parity on representative reads and mutations.
 assert f.cli(['helpdesk','get',other])['id']==other
 cli_note=f.cli(['helpdesk','reply',other,'--idempotency-key',key()],{'body':'CLI private note','internal':True})
 mcp=f.mcp('helpdesk_get',{'workspace_id':f.workspace,'ticket_id':other});assert not mcp.get('isError'),mcp
 mcp=f.mcp('helpdesk_reply',{'workspace_id':f.workspace,'ticket_id':other,'idempotency_key':key(),'input':{'body':'MCP support reply'}});assert not mcp.get('isError'),mcp
 absent(customer(other,other_code,'/messages'),cli_note['id'],'CLI private note')
 # Required audit failure rolls back the message and ticket version.
 before=get(other)
 sql("CREATE FUNCTION helpdesk.test_fail_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='helpdesk.message.created' THEN RAISE EXCEPTION 'fixture audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER helpdesk_test_audit BEFORE INSERT ON suite.audit_events FOR EACH ROW EXECUTE FUNCTION helpdesk.test_fail_audit()")
 try:call(hd+'/tickets/'+other+'/messages','POST',{'body':'Must roll back'},expected=503,key=key())
 finally:sql('DROP TRIGGER helpdesk_test_audit ON suite.audit_events; DROP FUNCTION helpdesk.test_fail_audit()')
 assert get(other)['version']==before['version'];absent(call(hd+'/tickets/'+other+'/messages'),'Must roll back')
 assert sql("SELECT count(*) FROM suite.idempotency_records WHERE operation_key LIKE 'helpdesk.%' AND (request::text LIKE '%"+note_text+"%' OR result::text LIKE '%"+note_text+"%')")=='0'
 # Database enforces message immutability for the runtime role.
 result=subprocess.run(['docker','compose','exec','-T','db','psql','-U','postgres','-d','postgres','-v','ON_ERROR_STOP=1'],input="SET ROLE looski_runtime; UPDATE helpdesk.messages SET body='changed' WHERE id='"+note['id']+"';",text=True,capture_output=True,cwd=ROOT,env=f.environment)
 assert result.returncode!=0 and 'permission denied' in result.stderr
 durations=[]
 for _ in range(30):
  start=time.perf_counter();customer(other,other_code);durations.append((time.perf_counter()-start)*1000)
 durations.sort();print('Customer ticket read, 30 sequential gateway requests: p50 %.1f ms; p95 %.1f ms'%(durations[15],durations[28]))
 print('PASS: standalone Helpdesk, customer isolation, private notes, scopes/revocation, retries, lifecycle, links/timeline, exports, audit rollback and CLI/MCP')
finally:
 for m,enabled in initial.items():module(m,enabled)

#!/usr/bin/env python3
"""Queued staff invitation mail and new-identity consent; isolated local SMTP."""
import secrets
from integration_fixture import Fixture
f=Fixture()
assert f.config['LOOSKI_MAIL_MODE']=='local'
base=f'/api/v1/workspaces/{f.workspace}/access/invitations'
def key():return secrets.token_hex(16)
role=f.call(f'/api/v1/workspaces/{f.workspace}/access/roles','POST',{'name':'Mail invite '+key(),'grants':[{'action':'workspace.read','scope':'workspace'}]},expected=201,key=key())
email='new-staff-'+key()+'@example.invalid'
# Do not use Fixture.call for secret-bearing responses: assertions must never print tokens.
import json
from urllib.request import Request,urlopen
from urllib.error import HTTPError
def call(path,body,status=200,retry=None):
 headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':retry or key()}
 try:r=urlopen(Request(f.base+path,data=json.dumps(body).encode(),headers=headers),timeout=20)
 except HTTPError as e:r=e
 with r:
  assert r.status==status,(path,r.status,status)
  return json.loads(r.read())
inv=call(base,{'email':email,'role_ids':[role['id']]},201)
job=call(base+'/'+inv['invitation']['id']+'/send',{'expected_version':1},202)
assert job['kind']=='workspace.invitation_email'
print('PASS: explicit invitation email is durably queued')
import subprocess,time,html,re,http.cookiejar
from urllib.parse import urlsplit,parse_qs,urlencode
from urllib.request import build_opener,HTTPCookieProcessor,HTTPRedirectHandler
from browser_forms import Forms

def compose(*args):
 return subprocess.run(['docker','compose',*args],env=f.environment,check=True,capture_output=True,text=True)
def status(job_id):return f.call(f'/api/v1/workspaces/{f.workspace}/jobs/'+job_id)
def wait_job(job_id):
 deadline=time.monotonic()+25
 while time.monotonic()<deadline:
  result=status(job_id)
  if result['state'] in ('completed','blocked','exhausted'):return result
  time.sleep(.15)
 raise AssertionError(('Delivery did not finish',result['state'],result['error_code']))
def mail_api(path):
 return json.loads(compose('exec','-T','auth','wget','-qO-','http://mailpit:8025/api/v1/'+path).stdout)
def message(recipient):
 matches=[m for m in mail_api('messages')['messages'] if any(t['Address']==recipient for t in m['To'])]
 assert matches,'Invitation mail did not arrive'
 return mail_api('message/'+matches[0]['ID'])
def create(recipient):return call(base,{'email':recipient,'role_ids':[role['id']]},201)
def once():
 compose('exec','-T','--user','10001','-e','LOOSKI_DATABASE_URL_FILE=/run/looski/database_url','-e','LOOSKI_JWT_SECRET_FILE=/run/looski/jwt_secret','-e','LOOSKI_INVITATION_KEY_FILE=/run/looski/invitation_key','api','looski','worker','--once')
def run_job(job_id):
 for _ in range(20):
  once();result=status(job_id)
  if result['attempts']>0:return result
 raise AssertionError('Job notification was not processed')
def wake(job_id):f.sql("SELECT pgmq.set_vt('looski_jobs',message_id,0) FROM suite.jobs WHERE id='"+job_id+"'")

assert wait_job(job['id'])['state']=='completed'
assert f.sql("SELECT email_confirmed_at IS NULL FROM auth.users WHERE email='"+email+"'")=='t'
assert f.sql("SELECT count(*) FROM suite.auth_identities i JOIN auth.users u ON i.subject=u.id::text WHERE u.email='"+email+"'")=='0'
mail=message(email);links=re.findall(r'href="([^"]+)"',mail['HTML']);assert len(links)==1
link=html.unescape(links[0]);parts=urlsplit(link)
assert parts.scheme+'://'+parts.netloc==f.base and parts.path=='/auth/confirm'
assert urlsplit(parse_qs(parts.query)['redirect_to'][0]).path=='/invitations/accept'
class NoRedirect(HTTPRedirectHandler):
 def redirect_request(self,*args,**kwargs):return None
jar=http.cookiejar.CookieJar();browser=build_opener(HTTPCookieProcessor(jar),NoRedirect())
def web(path,data=None,expected=200):
 headers={'Accept':'text/html'}
 if data is not None:headers['Origin']=f.base
 try:r=browser.open(Request(f.base+path,data=None if data is None else urlencode(data).encode(),headers=headers),timeout=20)
 except HTTPError as e:r=e
 with r:
  assert r.status==expected,(path.split('?')[0],r.status,expected)
  return r.read().decode(),r.headers
query={k:v[0] for k,v in parse_qs(parts.query).items()}
web(parts.path+'?'+urlencode({**query,'redirect_to':'https://attacker.invalid/invitations/accept?token=fake'}),expected=303)
assert not any(c.name=='looski_pending_invitation' for c in jar)
web(parts.path+'?'+parts.query,expected=303)
assert any(c.name=='looski_pending_invitation' for c in jar)
assert not any(c.name=='looski_access' for c in jar)
web('/auth/confirm');web('/auth/confirm',{},303)
password=secrets.token_urlsafe(24)
web('/auth/password',{'password':password,'confirm':password},303)
assert 'Review your pending workspace invitation' in web('/workspaces')[0]
assert f.sql("SELECT count(*) FROM suite.auth_identities i JOIN auth.users u ON i.subject=u.id::text WHERE u.email='"+email+"'")=='0'
content,_=web('/invitations/accept');assert 'Join Integration workspace' in content
fields=Forms(content).form('display_name');web('/invitations/accept',{**fields,'display_name':'New staff'},303)
assert 'Integration workspace' in web('/workspaces')[0]
print('PASS: new identity email proof, password setup and explicit invitation consent; no premature membership')

# Queued delivery retries an external failure without business grants or resetting passwords.
compose('stop','worker')
try:
 email2='retry-staff-'+key()+'@example.invalid';retry_inv=create(email2);path=base+'/'+retry_inv['invitation']['id']+'/send';retry_key=key()
 retry_job=call(path,{'expected_version':1},202,retry_key)
 assert call(path,{'expected_version':1},202,retry_key)['id']==retry_job['id']
 call(path,{'expected_version':1},409)
 compose('stop','mailpit')
 try:
  failed=run_job(retry_job['id']);assert failed['state']=='queued' and failed['error_code']=='provider' and failed['attempts']==1
 finally:compose('up','-d','--no-deps','--wait','mailpit')
 assert f.sql("SELECT count(*) FROM suite.auth_identities i JOIN auth.users u ON i.subject=u.id::text WHERE u.email='"+email2+"'")=='0'
 wake(retry_job['id']);once()
 assert status(retry_job['id'])['state']=='completed'
 assert message(email2)
 print('PASS: SMTP failure retains a retryable job and retries existing unconfirmed identity without granting access')
 # Email an already verified identity; password is still usable without opening the new link.
 another=create(email);another_job=call(base+'/'+another['invitation']['id']+'/send',{'expected_version':1},202)
 f.sql("UPDATE auth.users SET recovery_sent_at=now()-interval '2 minutes' WHERE email='"+email+"'")
 assert run_job(another_job['id'])['state']=='completed'
 web('/logout',{},303);web('/login',{'email':email,'password':password},303)
 assert 'Integration workspace' in web('/workspaces')[0]
 print('PASS: existing identity delivery leaves the current password and membership unchanged')
 # Revocation before execution blocks provider calls, leaving no new Auth account.
 stopped_email='revoked-staff-'+key()+'@example.invalid';stopped=create(stopped_email)
 stopped_job=call(base+'/'+stopped['invitation']['id']+'/send',{'expected_version':1},202)
 call(base+'/'+stopped['invitation']['id']+'/revoke',{'expected_version':1})
 blocked=run_job(stopped_job['id']);assert blocked['state']=='blocked' and blocked['error_code']=='unavailable'
 assert f.sql("SELECT count(*) FROM auth.users WHERE email='"+stopped_email+"'")=='0'
 print('PASS: worker observes current invitation revocation before external provisioning')
 # CLI/MCP delivery and typed job detail use the shared contract.
 cli_inv=create('cli-staff-'+key()+'@example.invalid')
 cli_job=f.cli(['access','send-invitation',cli_inv['invitation']['id'],'--idempotency-key',key()],{'expected_version':1})
 assert cli_job['kind']=='workspace.invitation_email'
 assert status(cli_job['id'])['invitation_context']['invitation_id']==cli_inv['invitation']['id']
 mcp_inv=create('mcp-staff-'+key()+'@example.invalid')
 assert not f.mcp('send_staff_invitation',{'workspace_id':f.workspace,'id':mcp_inv['invitation']['id'],'idempotency_key':key(),'change':{'expected_version':1}}).get('isError')
 print('PASS: CLI/MCP invitation delivery parity and separately authorized job context')
 browser_inv=create('browser-mail-'+key()+'@example.invalid')
 detail=f"/workspaces/{f.workspace}/access/invitations/{browser_inv['invitation']['id']}"
 with f.browser.open(f.base+detail) as response:fields=Forms(response.read().decode()).form('expected_version')
 with f.browser.open(Request(f.base+detail,data=urlencode({**fields,'intent':'send'}).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:
  assert 'Invitation email queued.' in response.read().decode()
 print('PASS: browser explicitly queues email delivery')
finally:compose('up','-d','--no-deps','--wait','worker')

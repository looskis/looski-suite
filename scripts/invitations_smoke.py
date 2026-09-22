#!/usr/bin/env python3
"""Staff invitation lifecycle through the public contract; isolated fixture only."""
import base64
import hashlib
import hmac
import json
import secrets
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request,urlopen
from urllib.error import HTTPError
from integration_fixture import Fixture

f=Fixture();base=f'/api/v1/workspaces/{f.workspace}/access/invitations'
def key():return secrets.token_hex(16)
def call(path,method='GET',body=None,status=200,token=None,retry=None):
    headers={'Authorization':'Bearer '+(token or f.token),'Content-Type':'application/json'}
    if retry:headers['Idempotency-Key']=retry
    try:r=urlopen(Request(f.base+path,method=method,headers=headers,data=None if body is None else json.dumps(body).encode()),timeout=20)
    except HTTPError as failure:r=failure
    with r:
        data=json.loads(r.read())
        # Responses may contain invitation tokens; never include them in assertions/logs.
        assert r.status==status,(method,path,r.status,status,data.get('code') if isinstance(data,dict) else None)
        return data

def auth(path,body,token=None):
    payload={'path':path,'body':body,'token':token}
    script="const x=JSON.parse(require('fs').readFileSync(0,'utf8')); fetch('http://auth:9999'+x.path,{method:'POST',headers:{'Content-Type':'application/json',...(x.token?{Authorization:'Bearer '+x.token}:{})},body:JSON.stringify(x.body)}).then(async r=>{if(!r.ok){console.error('Auth fixture failed',r.status);process.exit(1)}process.stdout.write(await r.text())})"
    return json.loads(subprocess.check_output(['docker','compose','exec','-T','web','node','-e',script],input=json.dumps(payload),text=True,env=f.environment))

def administrator():
    def b64(value):return base64.urlsafe_b64encode(value).rstrip(b'=')
    secret=(f.directory/'secrets/jwt_secret').read_bytes().strip()
    unsigned=b64(b'{"alg":"HS256","typ":"JWT"}')+b'.'+b64(json.dumps({'role':'service_role','iss':f.base+'/auth/v1','exp':int(time.time())+120}).encode())
    return (unsigned+b'.'+b64(hmac.new(secret,unsigned,hashlib.sha256).digest())).decode()

assert 'items' in call(base)
role=call(f'/api/v1/workspaces/{f.workspace}/access/roles','POST',{'name':'Invite '+key(),'grants':[{'action':'workspace.read','scope':'workspace'}]},status=201,retry=key())
email='invite-'+key()+'@example.invalid';password=secrets.token_urlsafe(24)
user=auth('/admin/users',{'email':email,'password':password,'email_confirm':True},administrator())
session=auth('/token?grant_type=password',{'email':email,'password':password})
assert call('/api/v1/workspaces',token=session['access_token'])==[]

def create(recipient=email,role_id=None):
    return call(base,'POST',{'email':recipient,'role_ids':[role_id or role['id']]},status=201,retry=key())
def accept(inv,token=None,status=200,retry=None):
    return call(base+'/'+inv['invitation']['id']+'/accept','POST',{'token':inv['token'],'display_name':'Invited colleague'},token=token or session['access_token'],status=status,retry=retry or key())

inv=create()
assert inv['invitation']['state']=='pending' and inv['token']
assert call(base+'/'+inv['invitation']['id'])==inv
# Authentication as a different real identity never grants the invitation's roles.
accept(inv,token=f.token,status=403)
# Stale JWT/email metadata is insufficient after current confirmation is removed.
f.sql("UPDATE auth.users SET email_confirmed_at=NULL WHERE id='"+user['id']+"'")
try:accept(inv,status=403)
finally:f.sql("UPDATE auth.users SET email_confirmed_at=now() WHERE id='"+user['id']+"'")
assert call('/api/v1/workspaces',token=session['access_token'])==[]
retry=key()
with ThreadPoolExecutor(max_workers=2) as pool:results=list(pool.map(lambda _:accept(inv,retry=retry),range(2)))
assert results[0]==results[1]
member=results[0]
assert member['state']=='active' and [r['id'] for r in member['roles']]==[role['id']]
assert call('/api/v1/workspaces',token=session['access_token'])[0]['id']==f.workspace
assert call(base+'/'+inv['invitation']['id'])['token'] is None
assert call(base+'/'+inv['invitation']['id'])['invitation']['state']=='accepted'
assert f.sql("SELECT count(*) FROM suite.audit_events WHERE record_id='"+inv['invitation']['id']+"' AND action='workspace.invitation.accepted'")=='1'
assert f.sql("SELECT count(*) FROM suite.idempotency_records WHERE result::text LIKE '%"+inv['token']+"%' OR request::text LIKE '%"+inv['token']+"%'")=='0'
print('PASS: invitation recipient proof, no implicit membership, atomic concurrent acceptance and secret-free replay')

# Previously accepted links cannot reactivate revoked membership or restore removed roles.
member=call(f'/api/v1/workspaces/{f.workspace}/access/members/'+member['id'],'PATCH',{'state':'removed','role_ids':[],'expected_version':member['version']},retry=key())
accept(inv,retry=retry,status=403)
assert call('/api/v1/workspaces',token=session['access_token'])==[]
# Pending invitation expiration/revocation and role version changes fail closed.
revoked=create();rid=revoked['invitation']['id']
result=call(base+'/'+rid+'/revoke','POST',{'expected_version':1},retry=key())
assert result['state']=='revoked' and call(base+'/'+rid)['token'] is None
accept(revoked,status=409)
expired=create();eid=expired['invitation']['id']
f.sql("UPDATE suite.staff_invitations SET created_at=now()-interval '2 days',expires_at=now()-interval '1 hour' WHERE id='"+eid+"'")
assert call(base+'/'+eid)['invitation']['state']=='expired'
accept(expired,status=409)
stale=create()
role=call(f'/api/v1/workspaces/{f.workspace}/access/roles/'+role['id'],'PATCH',{'name':role['name']+' edited','grants':role['grants'],'expected_version':role['version']},retry=key())
accept(stale,status=409)
print('PASS: removal is durable across acceptance retries; expiry, revocation and role changes deny pending grants')

# Late audit failure rolls back identity mapping, membership, role grants and acceptance.
email2='invite-'+key()+'@example.invalid';password2=secrets.token_urlsafe(24)
user2=auth('/admin/users',{'email':email2,'password':password2,'email_confirm':True},administrator())
session2=auth('/token?grant_type=password',{'email':email2,'password':password2})
atomic=create(email2)
f.sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:accept(atomic,token=session2['access_token'],status=503)
finally:f.sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert call('/api/v1/workspaces',token=session2['access_token'])==[]
assert call(base+'/'+atomic['invitation']['id'])['invitation']['state']=='pending'
assert f.sql("SELECT count(*) FROM suite.auth_identities WHERE subject='"+user2['id']+"'")=='0'
preview=call(base+'/'+atomic['invitation']['id']+'/preview','POST',{'token':atomic['token']},token=session2['access_token'])
assert preview['workspace_name']=='Integration workspace' and preview['invitation']['email']==email2
call(base+'/'+atomic['invitation']['id']+'/preview','POST',{'token':atomic['token']},token=f.token,status=403)
# Editing role grants is explicit; acceptance observes the original inviter's current authority.
owner_role=f.sql("SELECT r.id FROM suite.roles r JOIN suite.membership_roles mr ON mr.role_id=r.id WHERE r.workspace_id='"+f.workspace+"' AND r.key='owner' LIMIT 1")
f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+owner_role+"' AND permission_key='workspace.access.admin'")
try:accept(atomic,token=session2['access_token'],status=403)
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+owner_role+"','workspace.access.admin')")
print('PASS: late failure rolls back every acceptance write; previews and inviter revocation use current authority')

# Real CLI/MCP clients use the same invitation contract and protected token inputs.
assert f.cli(['access','invitation',atomic['invitation']['id']])['invitation']['id']==atomic['invitation']['id']
assert not f.mcp('get_staff_invitation',{'workspace_id':f.workspace,'id':atomic['invitation']['id']}).get('isError')
owner_token=f.token
try:
 f.token=session2['access_token']
 assert f.cli(['access','preview-invitation',atomic['invitation']['id']],{'token':atomic['token']})['invitation']['email']==email2
 assert not f.mcp('preview_staff_invitation',{'workspace_id':f.workspace,'id':atomic['invitation']['id'],'query':{'token':atomic['token']}}).get('isError')
finally:f.token=owner_token
print('PASS: CLI/MCP administration and recipient preview parity')

# The browser preserves an invitation through login without putting the token in rendered data.
import html
import http.cookiejar
import re
from urllib.parse import urlencode
from urllib.request import build_opener,HTTPCookieProcessor,HTTPRedirectHandler
class NoRedirect(HTTPRedirectHandler):
 def redirect_request(self,req,fp,code,msg,headers,newurl):return None
jar=http.cookiejar.CookieJar();browser=build_opener(HTTPCookieProcessor(jar),NoRedirect())
def web(path,status=200,data=None,origin=None):
 headers={'Accept':'text/html'}
 if data is not None:headers['Origin']=origin or f.base
 try:r=browser.open(Request(f.base+path,data=None if data is None else urlencode(data,doseq=True).encode(),headers=headers),timeout=20)
 except HTTPError as failure:r=failure
 with r:
  content=r.read().decode();assert r.status==status,(path.split('?')[0],r.status,status)
  return content,r.headers
path='/invitations/accept?'+urlencode({'workspace':f.workspace,'invitation':atomic['invitation']['id'],'token':atomic['token'],'next':'https://attacker.invalid'})
_,headers=web(path,303)
assert headers['Location']=='/invitations/accept' and headers['Referrer-Policy']=='no-referrer'
content,_=web('/invitations/accept');assert 'Sign in with the email address' in content and atomic['token'] not in content
web('/login',303,{'email':email2,'password':password2})
assert 'Review your pending workspace invitation' in web('/workspaces')[0]
content,_=web('/invitations/accept')
assert 'Join Integration workspace' in content and atomic['token'] not in content
from browser_forms import Forms
fields=Forms(content).form('display_name')
form={**fields,'display_name':'Browser colleague'}
web('/invitations/accept',403,form,'https://attacker.invalid')
web('/invitations/accept',400,{**form,'display_name':''})
web('/invitations/accept',303,form)
assert not any(c.name=='looski_pending_invitation' for c in jar)
assert 'Integration workspace' in web('/workspaces')[0]
print('PASS: browser pending-cookie capture, login return, recipient consent, CSRF and atomic acceptance')

# Admin browser can retain invalid drafts and revoke copyable invitations.
admin_path=f'/workspaces/{f.workspace}/access/invitations'
with f.browser.open(f.base+admin_path+'?'+urlencode({'role_ids':role['id']})) as response:
 content=response.read().decode();fields=Forms(content).form('email')
form={**fields,'email':'invalid-address','expires_in_hours':'72','role_ids':role['id']}
try:response=f.browser.open(Request(f.base+admin_path+'?'+urlencode({'role_ids':role['id']}),data=urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'}))
except HTTPError as failure:response=failure
with response:assert response.status==400 and 'invalid-address' in response.read().decode()
form['email']='browser-'+key()+'@example.invalid'
for invalid_expiry in ('banana','Infinity','','1.5','0','169'):
 invalid_form={**form,'expires_in_hours':invalid_expiry}
 try:response=f.browser.open(Request(f.base+admin_path,data=urlencode(invalid_form).encode(),headers={'Origin':f.base,'Accept':'text/html'}))
 except HTTPError as failure:response=failure
 with response:
  assert response.status==400,('invalid expiry accepted',invalid_expiry,response.status)
  rejected=Forms(response.read().decode()).form('email')
  assert rejected['expires_in_hours']==invalid_expiry and rejected['idempotency_key']==form['idempotency_key']
with f.browser.open(Request(f.base+admin_path,data=urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:
 content=response.read().decode();assert 'Private invitation link' in content
 detail_path='/'+response.geturl().split(f.base+'/',1)[1];fields=Forms(content).form('expected_version')
for bad_intent in ('','unexpected'):
 try:response=f.browser.open(Request(f.base+detail_path,data=urlencode({**fields,'intent':bad_intent}).encode(),headers={'Origin':f.base,'Accept':'text/html'}))
 except HTTPError as failure:response=failure
 with response:assert response.status==400
 assert call(base+'/'+detail_path.rsplit('/',1)[1])['invitation']['state']=='pending'
with f.browser.open(Request(f.base+detail_path,data=urlencode({**fields,'intent':'revoke'}).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:
 assert 'Invitation revoked.' in response.read().decode()
print('PASS: browser role selection, invitation draft preservation, private link and revocation')

# A wait on the workspace lock must not extend a pending invitation's lifetime.
expiry_inv=create(email2);expiry_id=expiry_inv['invitation']['id']
f.sql("UPDATE suite.staff_invitations SET expires_at=clock_timestamp()+interval '3 seconds' WHERE id='"+expiry_id+"'")
with ThreadPoolExecutor(max_workers=2) as pool:
 blocker=pool.submit(f.sql,"BEGIN; SET application_name='invitation-expiry-boundary'; SELECT id FROM suite.workspaces WHERE id='"+f.workspace+"' FOR NO KEY UPDATE; SELECT pg_sleep(5); COMMIT;")
 for _ in range(50):
  if f.sql("SELECT count(*) FROM pg_stat_activity WHERE application_name='invitation-expiry-boundary' AND wait_event='PgSleep'")=='1':break
  time.sleep(.05)
 else:raise AssertionError('Expiry blocker did not acquire lock')
 accept(expiry_inv,token=session2['access_token'],status=409)
 blocker.result()
assert call(base+'/'+expiry_id)['invitation']['state']=='expired'
print('PASS: pending invitation expiry is checked after lock waits')

# The distinct union, including overlapping grants, obeys the normal 50-role limit.
roles=[role['id']]
for n in range(50):
 roles.append(call(f'/api/v1/workspaces/{f.workspace}/access/roles','POST',{'name':'Invite bound '+key(),'grants':[{'action':'workspace.read','scope':'workspace'}]},status=201,retry=key())['id'])
full=call(base,'POST',{'email':email2,'role_ids':roles[:50]},status=201,retry=key())
full_member=accept(full,token=session2['access_token'])
assert len(full_member['roles'])==50
extra=create(email2,roles[50]);accept(extra,token=session2['access_token'],status=409)
assert call(base+'/'+extra['invitation']['id'])['invitation']['state']=='pending'
assert len(call(f'/api/v1/workspaces/{f.workspace}/access/members/'+full_member['id'])['roles'])==50
print('PASS: acceptance preserves overlapping roles and rejects a union above 50')

# Exercise real programmatic mutations, including retry and denial, not just schema presence.
def mcp_value(name,args):
 result=f.mcp(name,args);assert not result.get('isError'),name
 return json.loads(result['content'][0]['text'])
for transport in ('cli','mcp'):
 address=transport+'-invite-'+key()+'@example.invalid';secret=secrets.token_urlsafe(24)
 auth('/admin/users',{'email':address,'password':secret,'email_confirm':True},administrator())
 recipient=auth('/token?grant_type=password',{'email':address,'password':secret})['access_token']
 body={'email':address,'role_ids':[role['id']]}
 if transport=='cli':link=f.cli(['access','create-invitation','--idempotency-key',key()],body)
 else:link=mcp_value('create_staff_invitation',{'workspace_id':f.workspace,'idempotency_key':key(),'change':body})
 identifier=link['invitation']['id'];change={'token':link['token'],'display_name':transport+' colleague'};accept_key=key()
 assert f.mcp('accept_staff_invitation',{'workspace_id':f.workspace,'id':identifier,'idempotency_key':accept_key,'change':change}).get('isError')
 try:
  f.token=recipient
  if transport=='cli':
   granted=f.cli(['access','accept-invitation',identifier,'--idempotency-key',accept_key],change)
   assert f.cli(['access','accept-invitation',identifier,'--idempotency-key',accept_key],change)==granted
  else:
   args={'workspace_id':f.workspace,'id':identifier,'idempotency_key':accept_key,'change':change}
   granted=mcp_value('accept_staff_invitation',args);assert mcp_value('accept_staff_invitation',args)==granted
 finally:f.token=owner_token
 assert granted['state']=='active' and [r['id'] for r in granted['roles']]==[role['id']]
 revoked_link=create(address);identifier=revoked_link['invitation']['id']
 if transport=='cli':revoked=f.cli(['access','revoke-invitation',identifier,'--idempotency-key',key()],{'expected_version':1})
 else:revoked=mcp_value('revoke_staff_invitation',{'workspace_id':f.workspace,'id':identifier,'idempotency_key':key(),'change':{'expected_version':1}})
 assert revoked['state']=='revoked'
print('PASS: CLI/MCP creation, verified acceptance, retry, mismatched recipient denial and revocation')

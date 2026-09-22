#!/usr/bin/env python3
"""Role grants, member lifecycle and last-administrator acceptance."""
import secrets
from concurrent.futures import ThreadPoolExecutor
from integration_fixture import Fixture
f=Fixture();call,sql=f.call,f.sql
base='/api/v1/workspaces/'+f.workspace+'/access'
def key():return secrets.token_hex(16)
def grant(action,scope='workspace'):return {'action':action,'scope':scope}
def create(name,grants):return call(base+'/roles','POST',{'name':name,'grants':grants},expected=201,key=key())
def edit(role,**change):return call(base+'/roles/'+role['id'],'PATCH',{'name':role['name'],'grants':role['grants'],'expected_version':role['version'],**change},key=key())
catalog=call(base+'/permissions');assert any(p['action']=='workspace.access.admin' for p in catalog)
assert all(p['action']!='contacts.delete_permanently' for p in catalog),'Do not advertise unsupported grants'
name='Access '+secrets.token_hex(6)
role=create(name,[grant('workspace.read'),grant('contacts.read','assigned')])
assert role['version']==1 and not role['archived']
assert call(base+'/roles/'+role['id'])==role
body={'name':name+' updated','grants':[grant('contacts.read','assigned'),grant('contacts.read')],'expected_version':1}
k=key();updated=call(base+'/roles/'+role['id'],'PATCH',body,key=k);assert call(base+'/roles/'+role['id'],'PATCH',body,key=k)==updated
call(base+'/roles/'+role['id'],'PATCH',body,expected=409,key=key())
for unsupported in [[grant('workspace.access.admin','assigned')],[grant('unshipped.future')],[grant('contacts.delete_permanently')],[grant('contacts.read'),grant('contacts.read')]]:
 call(base+'/roles','POST',{'name':name,'grants':unsupported},expected=400,key=key())
# Final admin survives direct removal and role edits; no name-based privilege.
members=call(base+'/members')['items']
owner=next(m for m in members if m['state']=='active' and any(r['name']=='Owner' for r in m['roles']))
owner_role=call(base+'/roles/'+next(r['id'] for r in owner['roles'] if r['name']=='Owner'))
call(base+'/members/'+owner['id'],'PATCH',{'state':'removed','role_ids':[],'expected_version':owner['version']},expected=409,key=key())
call(base+'/roles/'+owner_role['id'],'PATCH',{'name':'Unprivileged name','grants':[],'expected_version':owner_role['version']},expected=409,key=key())
# Existing members may be suspended/removed/restored; removal clears old role assignments.
member=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Access fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
m=call(base+'/members/'+member)
m=call(base+'/members/'+member,'PATCH',{'state':'active','role_ids':[role['id']],'expected_version':m['version']},key=key())
assert [r['id'] for r in m['roles']]==[role['id']]
m=call(base+'/members/'+member,'PATCH',{'state':'removed','role_ids':[],'expected_version':m['version']},key=key());assert m['roles']==[]
m=call(base+'/members/'+member,'PATCH',{'state':'active','role_ids':[],'expected_version':m['version']},key=key());assert m['roles']==[]
archived=edit(updated,archived=True)
call(base+'/members/'+member,'PATCH',{'state':'active','role_ids':[role['id']],'expected_version':m['version']},expected=409,key=key())
# Access authority is checked from current assignments with the same still-valid token.
sql("DELETE FROM suite.role_permissions WHERE permission_key='workspace.access.admin' AND role_id='"+owner_role['id']+"'")
try:call(base+'/roles','POST',{'name':name,'grants':[]},expected=403,key=key())
finally:sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+owner_role['id']+"','workspace.access.admin')")
assert f.cli(['access','role',role['id']])['archived']
assert not f.mcp('list_workspace_roles',{'workspace_id':f.workspace,'query':{}}).get('isError')
print('PASS: supported grants, version/retry, last-administrator protection, member removal/restoration, current-token revocation and CLI/MCP')
# Removing access-read is as dangerous as removing the administrative action.
owner_role=call(base+'/roles/'+owner_role['id'])
call(base+'/roles/'+owner_role['id'],'PATCH',{'name':owner_role['name'],'grants':[g for g in owner_role['grants'] if g['action']!='workspace.access.read'],'expected_version':owner_role['version']},expected=409,key=key())
# Two effective administrative roles cannot concurrently remove the final authority.
backup=create(name+' backup',[grant(a) for a in ['workspace.read','workspace.access.read','workspace.access.admin']])
owner=call(base+'/members/'+owner['id'])
owner=call(base+'/members/'+owner['id'],'PATCH',{'state':'active','role_ids':[owner_role['id'],backup['id']],'expected_version':owner['version']},key=key())
def archive_race(role):
 from urllib.request import Request,urlopen
 from urllib.error import HTTPError
 import json
 req=Request(f.base+base+'/roles/'+role['id'],method='PATCH',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':key()},data=json.dumps({'name':role['name'],'grants':role['grants'],'expected_version':role['version'],'archived':True}).encode())
 try:response=urlopen(req,timeout=20)
 except HTTPError as failure:response=failure
 with response:return response.status
try:
 with ThreadPoolExecutor(max_workers=2) as pool:outcomes=list(pool.map(archive_race,[owner_role,backup]))
 assert sorted(outcomes)==[200,409],outcomes
finally:
 for r in [owner_role,backup]:
  current=call(base+'/roles/'+r['id'])
  if current['archived']:edit(current,archived=False)
 owner=call(base+'/members/'+owner['id'])
 call(base+'/members/'+owner['id'],'PATCH',{'state':'active','role_ids':[owner_role['id']],'expected_version':owner['version']},key=key())
# Workspace-qualified IDs and grant writes, audit rollback and bounded pagination.
import uuid
foreign=str(uuid.uuid4())
call(base+'/roles/'+foreign,expected=404)
call(base+'/members/'+member,'PATCH',{'state':'active','role_ids':[foreign],'expected_version':m['version']},expected=404,key=key())
page=call(base+'/roles?limit=1');assert len(page['items'])==1 and page['next_cursor']
assert call(base+'/roles?limit=1&after='+page['next_cursor'])['items'][0]['id']!=page['items'][0]['id']
count=sql('SELECT count(*) FROM suite.roles');sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:call(base+'/roles','POST',{'name':name+' rollback','grants':[]},expected=503,key=key())
finally:sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert sql('SELECT count(*) FROM suite.roles')==count
# Browser role creation, role update and membership update exercise CSRF and typed adapters.
from urllib.request import Request
from urllib.parse import urlencode
from html.parser import HTMLParser
class Fields(HTMLParser):
 def __init__(self):super().__init__();self.values={}
 def handle_starttag(self,tag,attrs):
  attrs=dict(attrs)
  if tag=='input' and attrs.get('type')=='hidden':self.values[attrs['name']]=attrs.get('value','')
web='/workspaces/'+f.workspace+'/access'
with f.browser.open(f.base+web) as response:
 fields=Fields();fields.feed(response.read().decode())
form={**fields.values,'name':name+' browser','grants':'contacts.read|assigned'}
with f.browser.open(Request(f.base+web,data=urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:
 browser_role=response.geturl().rsplit('/',1)[1];fields=Fields();fields.feed(response.read().decode())
form={**fields.values,'name':name+' renamed browser','grants':'contacts.read|workspace'}
with f.browser.open(Request(f.base+web+'/roles/'+browser_role,data=urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:assert 'renamed browser' in response.read().decode()
assert call(base+'/roles/'+browser_role)['grants']==[grant('contacts.read')]
with f.browser.open(f.base+web+'/members/'+member) as response:
 fields=Fields();fields.feed(response.read().decode())
form={**fields.values,'state':'suspended','role_ids':browser_role}
with f.browser.open(Request(f.base+web+'/members/'+member,data=urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:assert response.status==200
assert call(base+'/members/'+member)['state']=='suspended'
print('PASS: concurrent final-admin protection, read prerequisites, unknown IDs, pagination, atomic audit rollback and browser administration')
# Existing IDs in a different workspace must be rejected, not merely unknown IDs.
foreign_workspace=sql("INSERT INTO suite.workspaces(name) VALUES('Access foreign fixture') RETURNING id").splitlines()[0]
foreign_role=sql("INSERT INTO suite.roles(workspace_id,key,name) VALUES('"+foreign_workspace+"','foreign','Foreign') RETURNING id").splitlines()[0]
foreign_member=sql("INSERT INTO suite.memberships(workspace_id,user_id) VALUES('"+foreign_workspace+"','"+owner['user_id']+"') RETURNING id").splitlines()[0]
call(base+'/roles/'+foreign_role,expected=404)
call(base+'/members/'+foreign_member,expected=404)
m=call(base+'/members/'+member)
call(base+'/members/'+member,'PATCH',{'state':'active','role_ids':[foreign_role],'expected_version':m['version']},expected=404,key=key())
# Per-action scope unions do not turn broad read into broad write.
admin_only=create(name+' administration',[grant(a) for a in ['workspace.read','workspace.access.read','workspace.access.admin']])
assigned=create(name+' assigned',[grant('contacts.read','assigned'),grant('contacts.update','assigned')])
wide_read=create(name+' broad read',[grant('contacts.read')])
contacts='/api/v1/workspaces/'+f.workspace+'/contacts'
# Restore fixture membership to permit assignment as a current owner.
m=call(base+'/members/'+member);call(base+'/members/'+member,'PATCH',{'state':'active','role_ids':[],'expected_version':m['version']},key=key())
owned=call(contacts,'POST',{'display_name':name+' owned'},expected=201,key=key())
unassigned=call(contacts,'POST',{'display_name':name+' other','owner_membership_id':member},expected=201,key=key())
owner=call(base+'/members/'+owner['id'])
try:
 owner=call(base+'/members/'+owner['id'],'PATCH',{'state':'active','role_ids':[admin_only['id'],assigned['id'],wide_read['id']],'expected_version':owner['version']},key=key())
 assert call(contacts+'/'+unassigned['id'])['id']==unassigned['id']
 call(contacts+'/'+unassigned['id'],'PATCH',{'display_name':'Denied broad write','expected_version':1},expected=404,key=key())
 call(contacts+'/'+owned['id'],'PATCH',{'display_name':name+' own edit','expected_version':1},key=key())
 owner=call(base+'/members/'+owner['id'],'PATCH',{'state':'active','role_ids':[admin_only['id'],assigned['id']],'expected_version':owner['version']},key=key())
 call(contacts+'/'+unassigned['id'],expected=404)
finally:
 owner=call(base+'/members/'+owner['id'])
 call(base+'/members/'+owner['id'],'PATCH',{'state':'active','role_ids':[owner_role['id']],'expected_version':owner['version']},key=key())
from urllib.error import HTTPError
try:
 response=f.browser.open(Request(f.base+web,data=urlencode({'name':'Malformed','grants':'contacts.read|team','idempotency_key':key()}).encode(),headers={'Origin':f.base,'Accept':'text/html'}))
except HTTPError as failure:response=failure
with response:assert response.status==400,response.status
print('PASS: existing foreign-workspace IDs, same-action scope union, no cross-action escalation, immediate revocation and bounded browser validation')

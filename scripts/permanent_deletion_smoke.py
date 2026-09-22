#!/usr/bin/env python3
"""Controlled contact deletion through the public business boundary."""
from integration_fixture import Fixture
import secrets
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}/contacts'
def key():return secrets.token_hex(16)
input={'display_name':'Delete '+key(),'notes':'Temporary fixture','identifiers':[{'kind':'email','value':key()+'@example.invalid'}]}
creation_key=key()
contact=f.call(base,'POST',input,expected=201,key=creation_key)
f.call(base+'/'+contact['id']+'/archive','POST',{'expected_version':contact['version'],'archived':True},key=key())
contact=f.call(base+'/'+contact['id'])
result=f.call(base+'/'+contact['id']+'/delete-permanently','POST',{'expected_version':contact['version'],'confirm_id':contact['id']},key=key())
assert result['id']==contact['id']
f.call(base+'/'+contact['id'],expected=404)
print('PASS: separately authorized archived contact deletion')

from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request
from urllib.error import HTTPError
from urllib.parse import urlencode
from browser_forms import Forms

def create(identifiers=None,owner=None,shared=False):
 body={'display_name':'Delete fixture '+key(),'identifiers':identifiers or [],'allow_shared':shared}
 if owner:body['owner_membership_id']=owner
 return f.call(base,'POST',body,expected=201,key=key())
def get(c):return f.call(base+'/'+c['id'])
def archive(c):return f.call(base+'/'+c['id']+'/archive','POST',{'expected_version':get(c)['version'],'archived':True},key=key())
def delete(c,expected=200,retry=None,**overrides):
 return f.call(base+'/'+c['id']+'/delete-permanently','POST',{'expected_version':c['version'],'confirm_id':c['id'],**overrides},expected=expected,key=retry or key())

f.call(base,'POST',input,expected=409,key=creation_key)
assert f.sql("SELECT count(*) FROM contacts.identifiers WHERE normalized_value='"+input['identifiers'][0]['value']+"'")=='0'
assert f.sql("SELECT count(*) FROM suite.idempotency_records WHERE result->>'id'='"+contact['id']+"' AND operation_key='contacts.create'")=='0'
active=create();delete(active,409)
archived=archive(active);delete(archived,400,confirm_id='00000000-0000-0000-0000-000000000000')
delete(archived,409,expected_version=archived['version']-1)
owner_role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+owner_role+"' AND permission_key='contacts.delete_permanently'")
try:delete(archived,403)
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+owner_role+"','contacts.delete_permanently')")
retry=key()
with ThreadPoolExecutor(max_workers=2) as pool:results=list(pool.map(lambda _:delete(archived,retry=retry),range(2)))
assert results[0]==results[1]
assert f.sql("SELECT count(*) FROM suite.audit_events WHERE record_id='"+archived['id']+"' AND action='contacts.deleted_permanently'")=='1'
f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+owner_role+"' AND permission_key='contacts.delete_permanently'")
try:delete(archived,403,retry=retry)
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+owner_role+"','contacts.delete_permanently')")
print('PASS: explicit confirmation/version, active-record denial, independent deletion grant, concurrent retry and current replay authority')

# Assignment scope applies independently to both read and destructive authority.
owner_member=f.sql("SELECT membership_id FROM suite.membership_roles WHERE workspace_id='"+f.workspace+"' AND role_id='"+owner_role+"' LIMIT 1")
other=f.sql("SELECT id FROM suite.memberships WHERE workspace_id='"+f.workspace+"' AND state='active' AND id<>'"+owner_member+"' LIMIT 1")
assert other
foreign=archive(create(owner=other))
for action in ('contacts.read','contacts.delete_permanently'):
 f.sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE role_id='"+owner_role+"' AND permission_key='"+action+"'")
 try:delete(foreign,404)
 finally:f.sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE role_id='"+owner_role+"' AND permission_key='"+action+"'")
print('PASS: destructive action and read scopes intersect')

# A failed audit insert rolls back deletion, identifier cleanup, redaction and receipt.
atomic=archive(create([{'kind':'email','value':key()+'@example.invalid'}]));audit_key=key()
f.sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:delete(atomic,503,retry=audit_key)
finally:f.sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert get(atomic)==atomic
assert f.sql("SELECT count(*) FROM contacts.deletion_receipts WHERE id='"+atomic['id']+"'")=='0'
delete(atomic,retry=audit_key)
print('PASS: late audit failure leaves contact, identifiers and retry evidence intact')

# Shared identifiers belong to remaining contacts even after one contact is deleted.
identifier={'kind':'email','value':key()+'@example.invalid'}
first=create([identifier]);second=create([identifier],shared=True)
delete(archive(first));assert get(second)['identifiers'][0]['value']==identifier['value']
assert f.sql("SELECT count(*) FROM contacts.identifiers WHERE normalized_value='"+identifier['value']+"'")=='1'
delete(archive(second))
assert f.sql("SELECT count(*) FROM contacts.identifiers WHERE normalized_value='"+identifier['value']+"'")=='0'
print('PASS: owned identifier deletion retains shared identifiers and removes only orphans')

# Historical organization relationships and merged aliases are never cascaded away.
organization=f.call(f'/api/v1/workspaces/{f.workspace}/organizations','POST',{'name':'Retained '+key()},expected=201,key=key())
related=create();relation_path=base+'/'+related['id']+'/organizations/'+organization['id']
f.call(relation_path,'PUT',{'expected_contact_version':related['version']},key=key())
f.call(relation_path,'PUT',{'expected_contact_version':get(related)['version'],'ended':True},key=key())
related=archive(related);delete(related,409)
assert get(related)==related
assert f.call(f'/api/v1/workspaces/{f.workspace}/organizations/'+organization['id'])['name']==organization['name']
source=create();survivor=create()
survivor=f.call(base+'/'+survivor['id']+'/merge','POST',{'source_id':source['id'],'source_version':source['version'],'survivor_version':survivor['version'],'resolved':{'display_name':survivor['display_name']}},key=key())
source_version=int(f.sql("SELECT version FROM contacts.contacts WHERE id='"+source['id']+"'"));delete({**source,'version':source_version},409)
survivor=archive(survivor);delete(survivor,409)
assert get(source)['id']==survivor['id']
print('PASS: retained relationship history and merge aliases block deletion without destroying related records')

# All three clients call the same operation; browser confirmation remains explicit.
cli=archive(create());change={'expected_version':cli['version'],'confirm_id':cli['id']}
assert f.cli(['contacts','delete-permanently',cli['id'],'--idempotency-key',key()],change)['id']==cli['id']
mcp=archive(create());result=f.mcp('delete_contact_permanently',{'workspace_id':f.workspace,'contact_id':mcp['id'],'idempotency_key':key(),'change':{'expected_version':mcp['version'],'confirm_id':mcp['id']}})
assert not result.get('isError')
web=archive(create());path=f'/workspaces/{f.workspace}/contacts/'+web['id']+'/delete'
with f.browser.open(f.base+path) as r:fields=Forms(r.read().decode()).form('confirm_id')
for confirmation,status in [('',400),(web['id'],200)]:
 try:r=f.browser.open(Request(f.base+path,data=urlencode({**fields,'confirm_id':confirmation}).encode(),headers={'Origin':f.base,'Accept':'text/html'}))
 except HTTPError as e:r=e
 with r:assert r.status==status
f.call(base+'/'+web['id'],expected=404)
print('PASS: API/CLI/MCP/browser permanent-deletion parity and rejected empty confirmation')

# Force deletion to queue before an edit behind a held row; identifier-first ordering
# must prevent the edit/deletion inversion from becoming a database deadlock.
import json,time
from urllib.request import urlopen
race=archive(create([{'kind':'email','value':key()+'@example.invalid'}]))
def raw(path,method,body):
 try:r=urlopen(Request(f.base+path,method=method,data=json.dumps(body).encode(),headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':key()}),timeout=20)
 except HTTPError as e:r=e
 with r:return r.status,json.loads(r.read())
def wait_sql(statement,expected):
 deadline=time.monotonic()+3
 while time.monotonic()<deadline:
  if f.sql(statement)==expected:return
  time.sleep(.03)
 raise AssertionError('Concurrency fixture did not reach lock boundary')
with ThreadPoolExecutor(max_workers=3) as pool:
 blocker=pool.submit(f.sql,"BEGIN; SET application_name='deletion-row-boundary'; SELECT id FROM contacts.contacts WHERE id='"+race['id']+"' FOR UPDATE; SELECT pg_sleep(3); COMMIT;")
 wait_sql("SELECT count(*) FROM pg_stat_activity WHERE application_name='deletion-row-boundary' AND wait_event='PgSleep'",'1')
 removal=pool.submit(raw,base+'/'+race['id']+'/delete-permanently','POST',{'expected_version':race['version'],'confirm_id':race['id']})
 wait_sql("SELECT count(*) FROM pg_stat_activity WHERE usename='looski_runtime' AND wait_event_type='Lock'",'1')
 edit=pool.submit(raw,base+'/'+race['id'],'PATCH',{'expected_version':race['version'],'display_name':race['display_name']+' edited','identifiers':[{'kind':'email','value':race['identifiers'][0]['value']}]})
 wait_sql("SELECT count(*) FROM pg_stat_activity WHERE usename='looski_runtime' AND wait_event_type='Lock'",'2')
 blocker.result()
 deleted_result,edited_result=removal.result(),edit.result()
assert (deleted_result[0],edited_result[0]) in ((200,404),(409,200)),('Edit/deletion overlap',deleted_result[0],edited_result[0])
print('PASS: forced edit/deletion overlap obeys identifier-before-record locking without deadlock')

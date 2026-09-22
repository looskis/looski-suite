#!/usr/bin/env python3
"""Shared Files: real bytes, current authority, transactions and transport parity."""
import base64
import hashlib
import json
import secrets
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request
from integration_fixture import Fixture
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}';root=base+'/files'
def key():return secrets.token_hex(16)
def payload(content=b'\x00\xffimmutable\ncontent',**fields):return {'filename':'Résumé 雪 '+key()+'.bin','media_type':'application/octet-stream','content_base64':base64.b64encode(content).decode(),**fields}
def upload(body=None,retry=None,expected=201):return f.call(root,'POST',body or payload(),key=retry or key(),expected=expected)
def edit(file,expected=200,**fields):return f.call(root+'/'+file['id'],'PATCH',{'expected_version':file['version'],'filename':file['filename'],'media_type':file['media_type'],'notes':file['notes'],**fields},key=key(),expected=expected)
def archive(file,archived=True):return f.call(root+'/'+file['id']+'/archive','POST',{'expected_version':file['version'],'archived':archived},key=key())
def content(file,expected=200):return f.call(root+'/'+file['id']+'/content',expected=expected)
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
body=payload();retry=key()
with ThreadPoolExecutor(max_workers=2) as pool:results=list(pool.map(lambda _:upload(body,retry),range(2)))
file=results[0];assert file==results[1]
raw=base64.b64decode(body['content_base64']);assert file['byte_length']==len(raw) and file['sha256']==hashlib.sha256(raw).hexdigest()
assert content(file)=={'file':file,'content_base64':body['content_base64']}
upload({**body,'content_base64':base64.b64encode(b'different').decode()},retry,409)
assert f.sql("SELECT count(*) FROM files.contents WHERE file_id='"+file['id']+"'")=='1'
assert f.sql("SELECT count(*) FROM suite.audit_events WHERE record_id='"+file['id']+"' AND action='files.uploaded'")=='1'
assert f.sql("SELECT count(*) FROM suite.idempotency_records WHERE request::text LIKE '%content_base64%'")=='0'
print('PASS: concurrent upload replay preserves one immutable file and audit, with digest-only retry evidence')

for fields in [{'filename':'../escape'},{'filename':'bad\nname'},{'media_type':'text/plain; charset=utf-8'},{'content_base64':'aGk'},{'content_base64':'aGl='},{'notes':'x'*10001}]:upload(payload(**fields),expected=400)
empty=upload(payload(b''));assert content(empty)['content_base64']==''
large=bytes(range(256))*32768
large_file=upload(payload(large));assert base64.b64decode(content(large_file)['content_base64'])==large
upload(payload(large+b'x'),expected=400)
# Authentication rejects ordinary unauthenticated uploads; the pending-body unit test verifies read order.
f.call(root,'POST',{},authenticated=False,expected=401)
f.call(root,'POST',{'content_base64':'x'*(12*1024*1024)},key=key(),expected=413)
print('PASS: binary/empty/8 MiB fidelity, strict input, decoded/transport limits and authentication before body intake')

hidden_member=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Files hidden member') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
hidden=upload(payload(owner_membership_id=hidden_member))
for action in ['files.read','files.create','files.update','files.archive']:
 f.sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE role_id='"+role+"' AND permission_key='"+action+"'")
 try:
  if action=='files.read':
   content(hidden,404);assert hidden['id'] not in [r['id'] for r in f.call(root)['items']]
   f.call(root+'?after='+hidden['id'],expected=404)
   edit(hidden,404);upload(payload(owner_membership_id=hidden_member),expected=403)
  elif action=='files.create':upload(payload(owner_membership_id=hidden_member),expected=403)
  elif action=='files.update':edit(hidden,404);edit(file,403,owner_membership_id=hidden_member)
  else:f.call(root+'/'+hidden['id']+'/archive','POST',{'expected_version':1,'archived':True},key=key(),expected=404)
 finally:f.sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE role_id='"+role+"' AND permission_key='"+action+"'")
f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='files.create'")
try:upload(body,retry,403)
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','files.create')")
f.call('/api/v1/workspaces/00000000-0000-0000-0000-000000000000/files/'+file['id']+'/content',expected=403)
print('PASS: independent current read/create/edit/archive scope, hidden cursor, foreign workspace and revoked retry denial')

with ThreadPoolExecutor(max_workers=2) as pool:
 # Each writer uses a distinct replay key; exactly one can use this version.
 def race(n):
  try:return edit(file,filename='Concurrent '+str(n)+'.bin')
  except AssertionError as exc:
   assert "409" in str(exc),str(exc);return None
 results=list(pool.map(race,range(2)))
assert sum(r is not None for r in results)==1
file=next(r for r in results if r);assert file['version']==2
assert content(file)['content_base64']==body['content_base64']
file=archive(file);assert file['archived']
assert file['id'] not in [r['id'] for r in f.call(root)['items']]
assert file['id'] in [r['id'] for r in f.call(root+'?archived=true')['items']]
edit(file,409);assert content(file)['content_base64']==body['content_base64']
file=archive(file,False);assert not file['archived']
print('PASS: optimistic concurrent edits, archive/restore and permanent content fidelity')

before=f.sql('SELECT count(*) FROM files.files');bytes_before=f.sql('SELECT count(*) FROM files.contents');atomic=payload();atomic_key=key()
f.sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:upload(atomic,atomic_key,503)
finally:f.sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert f.sql('SELECT count(*) FROM files.files')==before and f.sql('SELECT count(*) FROM files.contents')==bytes_before
upload(atomic,atomic_key)
assert f.sql("SELECT has_column_privilege('looski_runtime','files.contents','content','UPDATE') OR has_table_privilege('looski_runtime','files.contents','DELETE')")=='f'
print('PASS: late audit failure rolls back bytes, metadata and retry evidence; runtime cannot mutate stored content')

# All-page metadata export and module independence.
selection={'collection':'files','limit':1};items=[]
while selection:
 page=f.call(base+'/exports/page','POST',selection);items+=page['items'];selection=page['next']
assert file in items and all('content_base64' not in item for item in items)
module=next(m for m in f.call(base+'/modules') if m['key']=='crm')
module=f.call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':False},key=key())
try:assert content(file)['file']==file
finally:f.call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
print('PASS: complete bounded file metadata export and Files while CRM is disabled')

cli_body=payload(b'CLI file');cli=f.cli(['files','upload','--idempotency-key',key()],cli_body)
assert f.cli(['files','get',cli['id']])==cli
with tempfile.TemporaryDirectory() as directory:
 path=Path(directory)/'download.bin'
 assert f.cli(['files','content',cli['id'],'--output',str(path)])==cli
 assert path.read_bytes()==b'CLI file' and path.stat().st_mode&0o777==0o600
assert json.loads(f.mcp('files_get',{'workspace_id':f.workspace,'id':file['id']})['content'][0]['text'])==file
mcp_body=payload(b'MCP bytes');mcp_result=f.mcp('files_upload',{'workspace_id':f.workspace,'idempotency_key':key(),'change':mcp_body});assert not mcp_result.get('isError')
mcp_file=json.loads(mcp_result['content'][0]['text'])
assert json.loads(f.mcp('files_content',{'workspace_id':f.workspace,'id':mcp_file['id']})['content'][0]['text'])['content_base64']==mcp_body['content_base64']
with f.browser.open(f.base+f'/workspaces/{f.workspace}/files') as response:
 assert 'Upload a file' in response.read().decode()
with f.browser.open(f.base+f'/workspaces/{f.workspace}/files/'+file['id']+'/download') as response:
 assert response.headers['Content-Disposition'].startswith('attachment;') and response.headers['Cache-Control']=='no-store' and response.headers['X-Content-Type-Options']=='nosniff'
 assert response.read()==raw
# Browser form writes are same-origin, versioned, and backed by the same business operation.
webpath=f.base+f'/workspaces/{f.workspace}/files/'+file['id']
with f.browser.open(Request(webpath,data=urlencode({'intent':'edit','expected_version':file['version'],'idempotency_key':key(),'filename':'Browser renamed.bin','media_type':'application/octet-stream','notes':'Browser notes'}).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:
 assert 'Browser renamed.bin' in response.read().decode()
try:f.browser.open(Request(webpath,data=urlencode({'intent':'archive','expected_version':file['version']+1,'idempotency_key':key()}).encode(),headers={'Origin':'https://different.invalid','Accept':'text/html'}))
except HTTPError as error:assert error.code==403
else:raise AssertionError('Cross-origin file mutation accepted')
print('PASS: API/CLI/MCP upload/read, private CLI artifact and safe browser download/edit/origin checks')

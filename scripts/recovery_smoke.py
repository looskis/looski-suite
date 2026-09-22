#!/usr/bin/env python3
"""Portable snapshot and isolated restore rehearsal on an explicit fixture stack."""
import argparse
import json
import secrets
from pathlib import Path
from integration_fixture import Fixture
from recovery import Installation,create,verify
from recovery_restore import restore
import recovery_archive as encryption

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--project',required=True)
parser.add_argument('--port',required=True,type=int)
parser.add_argument('--archive',type=Path)
parser.add_argument('--existing-target',type=Path)
args=parser.parse_args()
assert args.project.endswith('-identity-check')
f=Fixture();i=Installation(f.config)
directory=f.directory/'recovery-fixture';directory.mkdir(mode=0o700,exist_ok=True)
identity=directory/'identity'
recipient=encryption.recipient(identity) if identity.exists() else encryption.generate_identity(identity)
if args.archive:
 archive=args.archive
 job=None
else:
 archive=directory/(args.project+'.age')
 original=set(i.running());paused=['worker','web','auth']
 i.run('stop','worker')
 try:
  role=f.call(f'/api/v1/workspaces/{f.workspace}/access/roles','POST',{'name':'Recovery '+secrets.token_hex(6),'grants':[{'action':'workspace.read','scope':'workspace'}]},expected=201,key=secrets.token_hex(16))
  inv=f.call(f'/api/v1/workspaces/{f.workspace}/access/invitations','POST',{'email':'recovery-'+secrets.token_hex(6)+'@example.invalid','role_ids':[role['id']]},expected=201,key=secrets.token_hex(16))
  job=f.call(f'/api/v1/workspaces/{f.workspace}/access/invitations/'+inv['invitation']['id']+'/send','POST',{'expected_version':1},expected=202,key=secrets.token_hex(16))
  i.run('stop','web','auth')
  result=create(i,archive,recipient)
  assert not set(paused)&set(i.running()),'Backup started originally stopped services'
  print('PASS: portable backup includes initially stopped Auth/web/worker',json.dumps(result),flush=True)
 finally:
  i.run('start','--wait','--wait-timeout','60',*[s for s in paused if s in original])
manifest=verify(archive,identity)
assert manifest['metadata']['table_counts']['pgmq.q_looski_jobs']>0
print('PASS: authenticated payload includes live queue state and exact service images',flush=True)
result={'target':str(args.existing_target),'report':str(args.existing_target/'recovery-report.json')} if args.existing_target else restore(archive,identity,args.project,args.port)
print('PASS: isolated database, Files and queue restore',json.dumps(result),flush=True)
report=json.loads(Path(result['report']).read_text())
if job is None:job=next(j for j in report['quarantined_jobs'] if j['kind']=='workspace.invitation_email')
assert any(j['id']==job['id'] for j in report['quarantined_jobs'])
# Credential data stays in memory and is never part of diagnostic output.
from recovery_restore import preview
restored=preview(Path(result['target']))
from urllib.request import Request,urlopen,build_opener,HTTPCookieProcessor
from urllib.parse import urlencode
from urllib.error import HTTPError
import http.cookiejar,base64,hashlib
url=restored['url'];owner=json.loads((f.directory/'identity-fixture.json').read_text())
jar=http.cookiejar.CookieJar();browser=build_opener(HTTPCookieProcessor(jar))
with browser.open(Request(url+'/login',data=urlencode({'email':owner['email'],'password':owner['password']}).encode(),headers={'Origin':url,'Accept':'text/html'}),timeout=30) as response:assert response.status==200
new_token=next(c.value for c in jar if c.name=='looski_access')
def api(path,token=new_token,expected=200):
 try:r=urlopen(Request(url+path,headers={'Authorization':'Bearer '+token}),timeout=30)
 except HTTPError as error:r=error
 with r:
  assert r.status==expected,(path,r.status,expected)
  return json.loads(r.read())
api('/api/v1/workspaces',f.token,401)
workspaces=api('/api/v1/workspaces');assert any(w['id']==f.workspace for w in workspaces)
base='/api/v1/workspaces/'+f.workspace
for resource in ['contacts','organizations','crm/opportunities','crm/deals','tasks','files']:
 response=api(base+'/'+resource)
 assert response['items'],resource
files=api(base+'/files')['items'];content=api(base+'/files/'+files[0]['id']+'/content')
assert hashlib.sha256(base64.b64decode(content['content_base64'])).hexdigest()==content['file']['sha256']
assert api(base+'/jobs/'+job['id'])['state']=='blocked'
for path in ['/workspaces',f'/workspaces/{f.workspace}/contacts',f'/workspaces/{f.workspace}/files']:
 with browser.open(url+path,timeout=30) as response:assert response.status==200
print('PASS: restored password login, stable workspace identity, CRM/Tasks/Files reads, old-token denial and quarantined invitation; isolated loopback browser works',flush=True)
from recovery_journey import exercise
exercise(Path(result['target']),url,f.workspace,new_token,f.base+'/auth/v1')

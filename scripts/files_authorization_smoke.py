#!/usr/bin/env python3
"""Force permission revocation to overlap every Files mutation."""
import secrets,subprocess,time
from concurrent.futures import ThreadPoolExecutor
from integration_fixture import Fixture
from dev import ROOT
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}/files'
def key():return secrets.token_hex(16)
body={'filename':'Authorization race '+key()+'.txt','media_type':'text/plain','content_base64':'aGk='}
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
file=f.call(base,'POST',body,key=key(),expected=201)
for action in ['files.update','files.archive','files.create']:
 holder=subprocess.Popen(['docker','compose','exec','-T','db','psql','-U','postgres','-d','postgres','-At','-v','ON_ERROR_STOP=1'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1,cwd=ROOT,env=f.environment)
 try:
  holder.stdin.write("BEGIN; SELECT id FROM suite.workspaces WHERE id='"+f.workspace+"' FOR NO KEY UPDATE; DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='"+action+"'; SELECT 'locked';\n");holder.stdin.flush()
  while holder.stdout.readline().strip()!='locked':
   assert holder.poll() is None,'Lock holder failed'
  def mutation():
   if action=='files.create':return f.call(base,'POST',body,key=key(),expected=403)
   if action=='files.update':return f.call(base+'/'+file['id'],'PATCH',{'expected_version':file['version'],'filename':'Permission race changed','media_type':'text/plain'},key=key(),expected=403)
   return f.call(base+'/'+file['id']+'/archive','POST',{'expected_version':file['version'],'archived':True},key=key(),expected=403)
  with ThreadPoolExecutor(max_workers=1) as pool:
   result=pool.submit(mutation)
   time.sleep(0.4)
   holder.stdin.write('COMMIT;\n');holder.stdin.close();holder.wait(timeout=5)
   result.result(timeout=10)
 finally:
  if holder.poll() is None:holder.terminate();holder.wait(timeout=5)
  holder.stdout.close();holder.stderr.close()
  f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','"+action+"') ON CONFLICT DO NOTHING")
 print('PASS: '+action+' reauthorizes after the workspace lock and observes committed revocation')

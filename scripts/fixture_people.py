# SPDX-License-Identifier: AGPL-3.0-only
"""Real Auth members for scoped integration journeys; never prints credentials."""
import base64,hashlib,hmac,json,secrets,subprocess,time
from dev import ROOT

def people(f):
 call,sql=f.call,f.sql
 base='/api/v1/workspaces/'+f.workspace
 def key():return secrets.token_hex(16)
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
 return Person

"""Business writes and authorization on a restored, isolated fixture only."""
import base64
import hashlib
import hmac
import json
import secrets
import tempfile
import time
from urllib.error import HTTPError
from urllib.request import Request,urlopen
from recovery import Installation


def exercise(target,url,workspace,token,issuer):
    project=target.name
    assert project.endswith('-identity-check')
    values={'COMPOSE_PROJECT_NAME':project,'LOOSKI_HTTP_PORT':url.rsplit(':',1)[1],'LOOSKI_PUBLIC_URL':url,'LOOSKI_MAIL_MODE':'disabled','LOOSKI_DIAGNOSTICS_MODE':'disabled'}
    installation=Installation(values,target/'recipe');installation.compose+=['-f',str(target/'compose.recovery.json')]
    assert 'worker' not in installation.running()
    base='/api/v1/workspaces/'+workspace
    def key():return secrets.token_hex(16)
    def call(path,method='GET',body=None,expected=200,retry=None,identity=token):
        headers={'Authorization':'Bearer '+identity,'Content-Type':'application/json'}
        if retry:headers['Idempotency-Key']=retry
        try:response=urlopen(Request(url+path,method=method,headers=headers,data=None if body is None else json.dumps(body).encode()),timeout=30)
        except HTTPError as error:response=error
        with response:
            assert response.status==expected,(method,path,response.status,expected)
            return json.loads(response.read())
    def create(path,body):return call(base+path,'POST',body,201,key())
    name='Restored journey '+key()
    body={'display_name':name,'identifiers':[{'kind':'email','value':key()+'@example.invalid'}]};retry=key()
    contact=call(base+'/contacts','POST',body,201,retry)
    assert call(base+'/contacts','POST',body,201,retry)==contact
    change={'expected_version':contact['version'],'display_name':name+' edited','identifiers':body['identifiers']};retry=key()
    edited=call(base+'/contacts/'+contact['id'],'PATCH',change,retry=retry)
    assert call(base+'/contacts/'+contact['id'],'PATCH',change,retry=retry)==edited
    call(base+'/contacts/'+contact['id'],'PATCH',change,409,key())
    assert installation.sql("SELECT count(*) FROM suite.audit_events WHERE record_id='"+contact['id']+"'")=='2'
    print('PASS: restored create/edit, exact replay, stale-version denial and transactional audit',flush=True)

    # A real Auth identity receives only explicitly accepted workspace grants.
    def b64(value):return base64.urlsafe_b64encode(value).rstrip(b'=')
    secret=(installation.state/'secrets/jwt_secret').read_bytes().strip()
    unsigned=b64(b'{"alg":"HS256","typ":"JWT"}')+b'.'+b64(json.dumps({'role':'service_role','iss':issuer,'exp':int(time.time())+120}).encode())
    admin=(unsigned+b'.'+b64(hmac.new(secret,unsigned,hashlib.sha256).digest())).decode()
    def auth(path,body,bearer=None):
        script="const x=JSON.parse(require('fs').readFileSync(0,'utf8')); fetch('http://auth:9999'+x.path,{method:'POST',headers:{'Content-Type':'application/json',...(x.token?{Authorization:'Bearer '+x.token}:{})},body:JSON.stringify(x.body)}).then(async r=>{if(!r.ok){process.exit(1)}process.stdout.write(await r.text())})"
        with tempfile.TemporaryFile() as source:
            source.write(json.dumps({'path':path,'body':body,'token':bearer}).encode());source.seek(0)
            return json.loads(installation.run('exec','-T','web','node','-e',script,source=source))
    email='restored-'+key()+'@example.invalid';password=secrets.token_urlsafe(24)
    auth('/admin/users',{'email':email,'password':password,'email_confirm':True},admin)
    limited=auth('/token?grant_type=password',{'email':email,'password':password})['access_token']
    assert call('/api/v1/workspaces',identity=limited)==[]
    role=create('/access/roles',{'name':name,'grants':[{'action':'workspace.read','scope':'workspace'},{'action':'contacts.read','scope':'assigned'}]})
    invitation=create('/access/invitations',{'email':email,'role_ids':[role['id']]})
    call(base+'/access/invitations/'+invitation['invitation']['id']+'/accept','POST',{'token':invitation['token'],'display_name':name},retry=key(),identity=limited)
    call(base+'/contacts/'+contact['id'],expected=404,identity=limited)
    call(base+'/contacts','POST',body,403,key(),limited)
    call(base+'/crm/deals',expected=403,identity=limited)
    print('PASS: restored real identity, explicit membership consent, assigned-record invisibility and independent write/CRM denials',flush=True)

    category=create('/crm/categories',{'name':name})
    first=create('/crm/statuses',{'name':name+' initial','parent_id':category['id']})
    second=create('/crm/statuses',{'name':name+' active','parent_id':category['id']})
    deal=create('/crm/deals',{'title':name,'category_id':category['id'],'status_id':first['id']})
    path=base+'/crm/deals/'+deal['id']
    followup={'expected_version':deal['version'],'task':{'title':name+' follow-up'}};retry=key()
    link=call(path+'/followups','POST',followup,201,retry)
    assert call(path+'/followups','POST',followup,201,retry)==link
    assert call(base+'/tasks/'+link['task']['id'])==link['task']
    work=create('/tasks/templates',{'name':name,'title':name+' queued task','assignment':'initiator','due_offset_days':1})
    template=create('/crm/coordination-templates',{'name':name,'items':[{'label':'Follow up','task_template_revision_id':work['revision']['id']}]})
    create('/crm/coordination-rules',{'name':name,'status_id':second['id'],'revision_id':template['revision']['id']})
    change={'title':name,'category_id':category['id'],'status_id':second['id'],'expected_version':call(path)['version']};retry=key()
    counts=lambda:(installation.sql('SELECT count(*) FROM suite.jobs'),installation.sql('SELECT count(*) FROM pgmq.q_looski_jobs'))
    before=counts()
    installation.sql('REVOKE INSERT ON crm.coordination_runs FROM looski_runtime')
    try:call(path,'PATCH',change,503,retry)
    finally:installation.sql('GRANT INSERT ON crm.coordination_runs TO looski_runtime')
    assert counts()==before and call(path)['status_id']==first['id']
    transitioned=call(path,'PATCH',change,retry=retry)
    assert call(path,'PATCH',change,retry=retry)==transitioned
    assert tuple(int(x) for x in counts())==tuple(int(x)+1 for x in before)
    assert 'worker' not in installation.running()
    print('PASS: restored atomic follow-up, failed-transition rollback, durable PGMQ insertion and replay without outbound workers',flush=True)

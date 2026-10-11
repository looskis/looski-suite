#!/usr/bin/env python3
"""Browser-approved CLI/MCP login: PKCE code exchange, rotation, logout and revocation; isolated fixture only."""
import base64
import hashlib
import hmac
import http.cookiejar
import json
import os
import re
import secrets
import selectors
import subprocess
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import parse_qs, urlencode, urlsplit
from urllib.request import HTTPCookieProcessor, Request, build_opener, urlopen
from dev import ROOT
from integration_fixture import Fixture

f=Fixture();binary=str(ROOT/'target/debug/looski')
def key():return secrets.token_hex(16)
def b64(value):return base64.urlsafe_b64encode(value).rstrip(b'=').decode()
def pkce():
    verifier=b64(secrets.token_bytes(32));return verifier,b64(hashlib.sha256(verifier.encode()).digest())
def api(path,method='GET',body=None,token=None,status=200,retry=None):
    headers={'Content-Type':'application/json','X-Looski-Interface':'cli'}
    if token:headers['Authorization']='Bearer '+token
    if retry:headers['Idempotency-Key']=retry
    try:r=urlopen(Request(f.base+path,method=method,headers=headers,data=None if body is None else json.dumps(body).encode()),timeout=20)
    except HTTPError as failure:r=failure
    with r:
        raw=r.read();data=json.loads(raw) if raw else None
        # Bodies may contain codes/tokens; report only status and error code.
        assert r.status==status,(method,path,r.status,status,data.get('code') if isinstance(data,dict) else None)
        assert r.headers.get('Cache-Control')=='no-store'
        return data
def auth(path,body,token=None):
    payload={'path':path,'body':body,'token':token}
    script="const x=JSON.parse(require('fs').readFileSync(0,'utf8')); fetch('http://auth:9999'+x.path,{method:'POST',headers:{'Content-Type':'application/json',...(x.token?{Authorization:'Bearer '+x.token}:{})},body:JSON.stringify(x.body)}).then(async r=>{if(!r.ok){console.error('Auth fixture failed',r.status);process.exit(1)}process.stdout.write(await r.text())})"
    return json.loads(subprocess.check_output(['docker','compose','exec','-T','web','node','-e',script],input=json.dumps(payload),text=True,env=f.environment))
def administrator():
    secret=(f.directory/'secrets/jwt_secret').read_bytes().strip()
    unsigned=b64(b'{"alg":"HS256","typ":"JWT"}')+'.'+b64(json.dumps({'role':'service_role','iss':f.base+'/auth/v1','exp':int(time.time())+120}).encode())
    return unsigned+'.'+b64(hmac.new(secret,unsigned.encode(),hashlib.sha256).digest())

# A real colleague with an Auth account and a workspace membership.
access=f'/api/v1/workspaces/{f.workspace}/access'
role=api(access+'/roles','POST',{'name':'CLI login '+key(),'grants':[{'action':'workspace.read','scope':'workspace'},{'action':'contacts.read','scope':'workspace'}]},token=f.token,status=201,retry=key())
email='cli-'+key()+'@example.invalid';password=secrets.token_urlsafe(24)
auth('/admin/users',{'email':email,'password':password,'email_confirm':True},administrator())
direct=auth('/token?grant_type=password',{'email':email,'password':password})
invitation=api(access+'/invitations','POST',{'email':email,'role_ids':[role['id']]},token=f.token,status=201,retry=key())
member=api(access+'/invitations/'+invitation['invitation']['id']+'/accept','POST',{'token':invitation['token'],'display_name':'CLI colleague'},token=direct['access_token'],retry=key())
user_id=f.sql("SELECT user_id FROM suite.memberships WHERE id='"+member['id']+"'")

def browser_session():
    jar=http.cookiejar.CookieJar();browser=build_opener(HTTPCookieProcessor(jar))
    with browser.open(Request(f.base+'/login',data=urlencode({'email':email,'password':password}).encode(),headers={'Origin':f.base,'Accept':'text/html'}),timeout=20) as r:assert r.status==200 and r.geturl().endswith('/workspaces')
    return browser,jar
browser,jar=browser_session()
def cookie(name):return next(c.value for c in jar if c.name==name)

def query(challenge,state,device=None):
    q={'code_challenge':challenge,'code_challenge_method':'S256','state':state}
    if device:q['device']=device
    return urlencode(q)
def approve(challenge,state,device=None,opener=None,origin=None):
    opener=opener or browser;q=query(challenge,state,device)
    with opener.open(f.base+'/cli/authorize?'+q,timeout=20) as r:
        page=r.read().decode();assert 'Authorize the Looski CLI?' in page and email in page and r.headers.get('Cache-Control')=='no-store'
        # no-referrer would make browsers post Origin: null, which CSRF refuses.
        assert r.headers.get('Referrer-Policy')=='same-origin'
    form={'code_challenge':challenge,'code_challenge_method':'S256','state':state,**({'device':device} if device else {})}
    try:r=opener.open(Request(f.base+'/cli/authorize?/approve&'+q,data=urlencode(form).encode(),headers={'Origin':origin or f.base,'Accept':'text/html'}),timeout=20)
    except HTTPError as failure:return failure.status
    with r:
        page=r.read().decode();assert r.status==200 and r.headers.get('Cache-Control')=='no-store'
        found=re.search(r'aria-label="Authorization code"[^>]*>([^<]+)</output>',page);assert found,'Approval page must show the code'
        code,returned=found.group(1).split('#');assert returned==state
        return code

# Unauthenticated approval resumes after password sign-in; off-origin targets are ignored.
verifier,challenge=pkce();state=b64(secrets.token_bytes(16))
anonymous=build_opener(HTTPCookieProcessor(http.cookiejar.CookieJar()))
with anonymous.open(f.base+'/cli/authorize?'+query(challenge,state),timeout=20) as r:
    login_url=r.geturl();assert urlsplit(login_url).path=='/login' and parse_qs(urlsplit(login_url).query)['next']==['/cli/authorize?'+query(challenge,state)]
    assert 'authorizing the Looski CLI' in r.read().decode()
with anonymous.open(Request(login_url,data=urlencode({'email':email,'password':password}).encode(),headers={'Origin':f.base,'Accept':'text/html'}),timeout=20) as r:
    assert urlsplit(r.geturl()).path=='/cli/authorize' and 'Authorize the Looski CLI?' in r.read().decode()
for target in ['//evil.example/x','/\\evil.example','https://evil.example/','/logout']:
    other=build_opener(HTTPCookieProcessor(http.cookiejar.CookieJar()))
    with other.open(Request(f.base+'/login?'+urlencode({'next':target}),data=urlencode({'email':email,'password':password}).encode(),headers={'Origin':f.base,'Accept':'text/html'}),timeout=20) as r:
        assert urlsplit(r.geturl()).netloc==urlsplit(f.base).netloc and urlsplit(r.geturl()).path=='/workspaces',target
# Invalid requests and cross-origin approval are refused before any code exists.
try:browser.open(f.base+'/cli/authorize?'+query('short',state),timeout=20);raise AssertionError('Invalid challenge accepted')
except HTTPError as failure:assert failure.status==400
assert approve(challenge,state,origin='https://evil.example')==403
api('/api/v1/cli/authorizations','POST',{'code_challenge':challenge,'code_challenge_method':'plain'},token=cookie('looski_access'),status=400)
api('/api/v1/cli/authorizations','POST',{'code_challenge':challenge,'code_challenge_method':'S256'},status=401)
print('PASS: browser approval requires same-origin session, resumes after sign-in and rejects off-origin return targets')

def exchange(code,verifier,status=200):return api('/api/v1/cli/sessions','POST',{'code':code,'code_verifier':verifier},status=status)
# Full exchange mints a new, independent Auth session; the code is single use.
verifier,challenge=pkce();code=approve(challenge,b64(secrets.token_bytes(16)),'Smoke laptop')
assert f.sql("SELECT count(*) FROM suite.cli_authorizations WHERE code_hash=sha256(convert_to('"+code+"','UTF8')) AND device_label='Smoke laptop'")=='1'
assert f.sql("SELECT count(*) FROM suite.security_events WHERE details::text LIKE '%"+code+"%'")=='0'
session=exchange(code,verifier);assert session['user']['email']==email and session['refresh_token'] and session['expires_in']>0
assert exchange(code,verifier,400)['code']=='invalid_grant'
assert [w['id'] for w in api('/api/v1/workspaces',token=session['access_token'])]==[f.workspace]
assert json.loads(base64.urlsafe_b64decode(session['access_token'].split('.')[1]+'=='))['session_id']!=json.loads(base64.urlsafe_b64decode(cookie('looski_access').split('.')[1]+'=='))['session_id']
# A wrong verifier burns the code; the right verifier cannot use it afterwards.
verifier,challenge=pkce();code=approve(challenge,b64(secrets.token_bytes(16)))
exchange(code,pkce()[0],400);exchange(code,verifier,400)
# Expiry is checked against the clock at exchange time.
verifier,challenge=pkce();code=approve(challenge,b64(secrets.token_bytes(16)))
f.sql("UPDATE suite.cli_authorizations SET created_at=now()-interval '10 minutes',expires_at=now()-interval '5 minutes' WHERE code_hash=sha256(convert_to('"+code+"','UTF8'))")
exchange(code,verifier,400)
# A pending code stops working when the account is suspended before exchange.
verifier,challenge=pkce();code=approve(challenge,b64(secrets.token_bytes(16)))
f.sql("UPDATE suite.users SET state='suspended' WHERE id='"+user_id+"'")
try:exchange(code,verifier,400)
finally:f.sql("UPDATE suite.users SET state='active' WHERE id='"+user_id+"'")
exchange('unknown-'+key(),verifier,400)
events=f.sql("SELECT string_agg(action||':'||source_interface,',' ORDER BY occurred_at,action) FROM suite.security_events WHERE user_id='"+user_id+"'").split(',')
assert events.count('cli_login.authorized:web')==4 and events.count('cli_login.exchanged:cli')==1 and events.count('cli_login.rejected:cli')==2,events
assert f.sql("SELECT string_agg(outcome,',' ORDER BY created_at) FROM suite.cli_authorizations WHERE user_id='"+user_id+"' AND consumed_at IS NOT NULL")=='issued,rejected,rejected'
print('PASS: PKCE exchange, single use, wrong verifier, expiry, suspension and hashed-only storage with security audit')

# Refresh rotates. Auth tolerates the immediate parent (a lost response), but a
# replaced token older than that fails after the reuse interval and revokes the family.
rotated=api('/api/v1/cli/sessions/refresh','POST',{'refresh_token':session['refresh_token']})
assert rotated['refresh_token']!=session['refresh_token'] and api('/api/v1/workspaces',token=rotated['access_token'])
latest=api('/api/v1/cli/sessions/refresh','POST',{'refresh_token':rotated['refresh_token']})
time.sleep(11)
assert api('/api/v1/cli/sessions/refresh','POST',{'refresh_token':session['refresh_token']},status=400)['code']=='invalid_grant'
api('/api/v1/cli/sessions/refresh','POST',{'refresh_token':latest['refresh_token']},status=400)
# Browser-created sessions cannot be refreshed through the public CLI endpoint.
other=auth('/token?grant_type=password',{'email':email,'password':password})
api('/api/v1/cli/sessions/refresh','POST',{'refresh_token':other['refresh_token']},status=400)
assert api('/api/v1/workspaces',token=cookie('looski_access'))
print('PASS: refresh rotation, replaced-token rejection and CLI-only refresh')

# The real CLI: printed link, pasted code, private stored login, whoami and MCP.
config=tempfile.TemporaryDirectory(dir=f.directory);environment={**f.environment,'LOOSKI_CONFIG_DIR':config.name,'LOOSKI_CREDENTIAL_STORE':'file','LOOSKI_API_URL':f.base}
environment.pop('LOOSKI_ACCESS_TOKEN_FILE',None)
def cli(*arguments,check=True):
    result=subprocess.run([binary,*arguments],env=environment,capture_output=True,text=True,timeout=60)
    assert not check or result.returncode==0,(arguments,result.returncode,result.stderr[-300:])
    return result
def login(paste=None):
    process=subprocess.Popen([binary,'login','--no-browser','--device-label','Smoke terminal'],env=environment,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    selector=selectors.DefaultSelector();selector.register(process.stderr,selectors.EVENT_READ);printed=''
    # The prompt has no trailing newline; read raw bytes as they arrive.
    while '/cli/authorize?' not in printed or 'Paste the authorization code' not in printed:
        assert selector.select(20),'CLI did not print a link'
        chunk=os.read(process.stderr.fileno(),4096).decode();assert chunk,'CLI exited early';printed+=chunk
    link=urlsplit(re.search(r'(http\S+/cli/authorize\?\S+)',printed).group(1));params={k:v[0] for k,v in parse_qs(link.query).items()}
    assert link.netloc==urlsplit(f.base).netloc and params['code_challenge_method']=='S256' and params['device']=='Smoke terminal'
    code=approve(params['code_challenge'],params['state'],params['device'])
    out,err=process.communicate((paste(code,params['state']) if paste else code+'#'+params['state'])+'\n',timeout=60)
    return process.returncode,out,err
code,out,err=login(lambda code,state:code+'#'+b64(secrets.token_bytes(16)))
assert code!=0 and 'different sign-in request' in err and not list(Path(config.name).glob('sessions/*.json'))
code,out,err=login();assert code==0,err[-300:]
status=json.loads(out);assert status['user']['email']==email and status['credential_store']=='file' and 'access_token' not in out
stored=list(Path(config.name).glob('sessions/*.json'));assert len(stored)==1 and stored[0].stat().st_mode&0o777==0o600 and Path(config.name,'sessions').stat().st_mode&0o777==0o700
whoami=json.loads(cli('whoami').stdout);assert whoami['user']['email']==email and [w['id'] for w in whoami['workspaces']]==[f.workspace]
assert 'items' in json.loads(cli('contacts','--workspace',f.workspace,'list').stdout)
def mcp(name,arguments):
    process=subprocess.Popen([binary,'mcp'],env=environment,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,bufsize=1)
    selector=selectors.DefaultSelector();selector.register(process.stdout,selectors.EVENT_READ)
    def rpc(method,params=None,identifier=None):
        message={'jsonrpc':'2.0','method':method,**({'params':params} if params is not None else {}),**({'id':identifier} if identifier is not None else {})}
        process.stdin.write(json.dumps(message)+'\n');process.stdin.flush()
        if identifier is None:return
        while True:
            assert selector.select(20),'MCP timed out';reply=json.loads(process.stdout.readline())
            if reply.get('id')==identifier:return reply['result']
    try:
        rpc('initialize',{'protocolVersion':'2025-11-25','capabilities':{},'clientInfo':{'name':'cli-login-smoke','version':'1'}},1);rpc('notifications/initialized')
        return rpc('tools/call',{'name':name,'arguments':arguments},2)
    finally:
        process.stdin.close();process.wait(timeout=10);selector.close()
result=mcp('list_workspaces',{});assert not result.get('isError') and json.loads(result['content'][0]['text'])[0]['id']==f.workspace
print('PASS: CLI login prints link and accepts pasted code, rejects foreign state, stores privately; whoami/CLI/MCP use stored login')

# Concurrent CLI and MCP processes with an expiring token refresh exactly once
# under the shared lock; reuse would revoke the whole session family.
def expire():
    path=stored[0];data=json.loads(path.read_text());data['expires_at']=0;path.write_text(json.dumps(data))
    return data['refresh_token']
before=expire()
with ThreadPoolExecutor(max_workers=6) as pool:
    outcomes=list(pool.map(lambda i:cli('whoami',check=False).returncode if i else (0 if not mcp('list_workspaces',{}).get('isError') else 1),range(6)))
assert outcomes==[0]*6,outcomes
assert json.loads(stored[0].read_text())['refresh_token']!=before
time.sleep(11);expire();assert cli('whoami').returncode==0
# A token the API rejects is refreshed once and the request retried.
data=json.loads(stored[0].read_text());data['access_token']='rejected.'+data['access_token'];data['expires_at']=int(time.time())+3600;stored[0].write_text(json.dumps(data))
assert cli('whoami').returncode==0 and not json.loads(stored[0].read_text())['access_token'].startswith('rejected.')
cli_token=json.loads(stored[0].read_text())['access_token']
print('PASS: concurrent CLI/MCP refresh under the cross-process lock without refresh-token reuse')

# Logout revokes only the CLI session; the browser stays signed in.
cli_refresh=json.loads(stored[0].read_text())['refresh_token']
out=json.loads(cli('logout').stdout);assert out['server_revoked'] and out['was_signed_in'] and not list(Path(config.name).glob('sessions/*.json'))
api('/api/v1/workspaces',token=cli_token,status=401)
api('/api/v1/cli/sessions/refresh','POST',{'refresh_token':cli_refresh},status=400)
with browser.open(f.base+'/workspaces',timeout=20) as r:assert urlsplit(r.geturl()).path=='/workspaces'
assert api('/api/v1/workspaces',token=cookie('looski_access'))
result=cli('whoami',check=False);assert result.returncode==4 and 'looski login' in result.stderr
result=mcp('list_workspaces',{});assert result['isError'] and 'looski login' in result['content'][0]['text'] and result['structuredContent']['code']=='unauthenticated'
assert 'cli_session.logout:cli' in f.sql("SELECT string_agg(action||':'||source_interface,',') FROM suite.security_events WHERE user_id='"+user_id+"'")
print('PASS: logout revokes the CLI session but not the browser session; MCP/CLI then ask for looski login')

# Removing the membership denies workspace access to a still-valid CLI login.
code,out,err=login();assert code==0,err[-300:]
current=api(access+'/members/'+member['id'],token=f.token)
api(access+'/members/'+member['id'],'PATCH',{'state':'removed','role_ids':[],'expected_version':current['version']},token=f.token,retry=key())
assert json.loads(cli('whoami').stdout)['workspaces']==[]
assert cli('contacts','--workspace',f.workspace,'list',check=False).returncode==4
assert mcp('list_contacts',{'workspace_id':f.workspace,'query':{}})['isError']
cli('logout');config.cleanup()
# Outstanding approvals are bounded per user.
verifiers=[pkce() for _ in range(6)];token=cookie('looski_access')
statuses=[]
for _,challenge in verifiers:
    try:urlopen(Request(f.base+'/api/v1/cli/authorizations',method='POST',headers={'Authorization':'Bearer '+token,'Content-Type':'application/json'},data=json.dumps({'code_challenge':challenge,'code_challenge_method':'S256'}).encode()),timeout=20).close();statuses.append(201)
    except HTTPError as failure:statuses.append(failure.status)
assert statuses.count(429)>=1 and statuses[0]==201,statuses
print('PASS: membership removal denies a live CLI login; outstanding approvals are bounded')

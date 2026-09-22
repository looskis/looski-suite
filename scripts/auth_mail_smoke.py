#!/usr/bin/env python3
"""Authentication email browser journey, isolated local-mail installation only."""
import http.cookiejar
import json
from urllib.error import HTTPError
from urllib.request import Request, build_opener, HTTPCookieProcessor, HTTPRedirectHandler
from urllib.parse import urlencode
from integration_fixture import Fixture

f=Fixture()
class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self,req,fp,code,msg,headers,newurl): return None
jar=http.cookiejar.CookieJar()
browser=build_opener(HTTPCookieProcessor(jar),NoRedirect())
def request(path,status=200,data=None,origin=None):
    headers={'Accept':'text/html'}
    if data is not None: headers['Origin']=origin or f.base
    req=Request(f.base+path,data=None if data is None else urlencode(data).encode(),headers=headers)
    try: response=browser.open(req,timeout=20)
    except HTTPError as failure: response=failure
    with response:
        content=response.read().decode()
        assert response.status==status,(path.split('?')[0],response.status,status)
        return content,response.headers

# The public page must expose both supported email journeys.
content,_=request('/auth/email')
assert 'Email me a sign-in link' in content
assert 'Reset my password' in content
print('PASS: authentication email entry points')

import html
import re
import secrets
import subprocess
import time
from urllib.parse import urlsplit, parse_qs

assert f.config['LOOSKI_MAIL_MODE']=='local','Use LOOSKI_MAIL_MODE=local'
owner=json.loads((f.directory/'identity-fixture.json').read_text())
email=owner['email']
def mail_api(path):
    data=subprocess.check_output(['docker','compose','exec','-T','auth','wget','-qO-','http://mailpit:8025/api/v1/'+path],cwd=f.directory.parents[1],env=f.environment)
    return json.loads(data)
def messages():
    return [m for m in mail_api('messages')['messages'] if any(to['Address']==email for to in m['To'])]
def request_link(kind, fallback=False):
    # Advance only this isolated fixture's last-send time, avoiding minute-long waits.
    f.sql("UPDATE auth.users SET recovery_sent_at=now()-interval '2 minutes' WHERE email='"+email+"'")
    before={m['ID'] for m in messages()}
    body,_=request('/auth/email',data={'email':email,'kind':kind})
    assert 'If this address has an eligible account' in body
    deadline=time.monotonic()+10
    while time.monotonic()<deadline:
        added=[m for m in messages() if m['ID'] not in before]
        if added: break
        time.sleep(.1)
    assert added,'SMTP delivery did not arrive'
    message=mail_api('message/'+added[0]['ID'])
    links=re.findall(r'href="([^"]+)"',message['HTML'])
    assert len(links)==1,'Unexpected template links'
    link=html.unescape(links[0]);parts=urlsplit(link)
    assert parts.scheme+'://'+parts.netloc==f.base
    assert parts.path==('/auth/v1/verify' if fallback else '/auth/confirm')
    assert parse_qs(parts.query)['type']==[kind]
    return parts.path+'?'+parts.query

def open_link(link):
    _,headers=request(link,303)
    assert headers['Location']=='/auth/confirm'
    assert headers['Referrer-Policy']=='no-referrer'
    assert 'no-store' in headers['Cache-Control']
    cookies=headers.get_all('Set-Cookie')
    assert cookies and all('HttpOnly' in c and 'SameSite=Lax' in c for c in cookies)
    body,headers=request('/auth/confirm')
    assert 'Continue only if you requested this email' in body
    query=parse_qs(urlsplit(link).query)
    assert query.get('token_hash',query.get('token'))[0] not in body
    assert not any(c.name=='looski_access' for c in jar)

request('/auth/email',403,{'email':email,'kind':'magiclink'},'https://attacker.invalid')
link=request_link('magiclink')
open_link(link)
# GET previews are safe; another preview does not consume the link.
request('/auth/confirm')
request('/workspaces',303)
request('/auth/confirm',403,{},'https://attacker.invalid')
_,headers=request('/auth/confirm',303,{})
assert headers['Location']=='/workspaces'
assert 'Integration workspace' in request('/workspaces')[0]
assert not any(c.name=='looski_pending_link' for c in jar)
request(link,303)
request('/auth/confirm',400,{})
# Throttled requests deliberately use the same account-neutral response.
count=len(messages())
body,_=request('/auth/email',data={'email':email,'kind':'magiclink'})
assert 'If this address has an eligible account' in body and len(messages())==count
request('/logout',303,{})
print('PASS: delivered magic link, clean callback, preview safety, reuse, throttling and CSRF')

link=request_link('recovery')
open_link(link)
_,headers=request('/auth/confirm',303,{})
assert headers['Location']=='/auth/password'
assert 'Set your password' in request('/auth/password')[0]
request('/auth/password',403,{'password':'x'*20,'confirm':'x'*20},'https://attacker.invalid')
request('/auth/password',400,{'password':'short','confirm':'short'})
replacement=secrets.token_urlsafe(24)
changed=False
try:
    request('/auth/password',303,{'password':replacement,'confirm':replacement});changed=True
    request('/logout',303,{})
    request('/login',400,{'email':email,'password':owner['password']})
    request('/login',303,{'email':email,'password':replacement})
    assert 'Integration workspace' in request('/workspaces')[0]
finally:
    if changed:
        # Keep the shared private fixture credentials usable for subsequent suites.
        request('/login',303,{'email':email,'password':replacement})
        request('/auth/password',303,{'password':owner['password'],'confirm':owner['password']})
request('/logout',303,{})
request('/auth/password',303)
print('PASS: password recovery, replacement login, old-password denial and unchanged membership')

link=request_link('magiclink')
f.sql("UPDATE auth.users SET recovery_sent_at=now()-interval '2 hours' WHERE email='"+email+"'")
open_link(link)
request('/auth/confirm',400,{})
request('/workspaces',303)
request('/auth/confirm?token_hash=bad&type=magiclink',400)
missing='missing-'+secrets.token_hex(5)+'@example.invalid'
for kind in ['magiclink','recovery']:
    body,_=request('/auth/email',data={'email':missing,'kind':kind})
    assert 'If this address has an eligible account' in body
assert f.sql("SELECT count(*) FROM auth.users WHERE email='"+missing+"'")=='0'
request('/auth/v1/verify?token=secret&type=magiclink',400)
request('/auth/v1/verify',404,{})
print('PASS: expired/invalid links, no account enumeration or implicit signup, private raw Auth')

# Delivery failure is account-neutral; operator diagnostics exclude provider details.
subprocess.run(['docker','compose','stop','mailpit'],cwd=f.directory.parents[1],env=f.environment,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
try:
    f.sql("UPDATE auth.users SET recovery_sent_at=now()-interval '2 minutes' WHERE email='"+email+"'")
    body,_=request('/auth/email',200,{'email':email,'kind':'magiclink'})
    absent,_=request('/auth/email',200,{'email':missing,'kind':'magiclink'})
    assert body==absent and 'If this address has an eligible account' in body
finally:
    subprocess.run(['docker','compose','up','-d','--wait','mailpit'],cwd=f.directory.parents[1],env=f.environment,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
print('PASS: SMTP outage is account-neutral, recoverable and secret-free')

# A cold Auth cache plus a template outage uses the provider default template.
# Its compatibility URL still enters the explicit-confirmation BFF, never raw Auth.
def compose(*args):
    subprocess.run(['docker','compose',*args],cwd=f.directory.parents[1],env=f.environment,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
def auth_ready():
    deadline=time.monotonic()+30
    while time.monotonic()<deadline:
        try:
            request('/auth/v1/health')
            return
        except (AssertionError,OSError): time.sleep(.2)
    raise AssertionError('Auth did not recover')
compose('stop','auth-templates')
try:
    compose('restart','auth');auth_ready()
    for kind in ['magiclink','recovery']:
        link=request_link(kind,fallback=True)
        open_link(link)
        request('/auth/confirm',303,{})
        assert 'Integration workspace' in request('/workspaces')[0]
        request('/logout',303,{})
finally:
    compose('up','-d','--wait','auth-templates')
    compose('restart','auth');auth_ready()
print('PASS: cold-cache template outage retains safe, usable magic-link/recovery callbacks')

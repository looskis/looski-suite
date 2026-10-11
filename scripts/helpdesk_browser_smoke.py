#!/usr/bin/env python3
"""Real gateway browser-form journey, no scripting: public intake/code/replies and staff notes."""
import http.cookiejar,json,re,secrets
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request,build_opener,HTTPCookieProcessor,HTTPRedirectHandler
from forms_browser_html import Forms
from integration_fixture import Fixture
f=Fixture();api=f'/api/v1/workspaces/{f.workspace}';root=f.base+f'/workspaces/{f.workspace}/helpdesk'
jar=http.cookiejar.CookieJar();customer=build_opener(HTTPCookieProcessor(jar));other=build_opener(HTTPCookieProcessor())
def key():return secrets.token_hex(16)
def fetch(opener,url,data=None,expected=200,origin=None):
 request=Request(url,data=None if data is None else urlencode(data).encode(),headers={'Accept':'text/html',**({'Origin':origin or f.base} if data is not None else {})})
 try:r=opener.open(request,timeout=30)
 except HTTPError as e:r=e
 with r:
  body=r.read().decode();assert r.status==expected,(url,r.status,expected,body[:500]);assert r.headers.get('Cache-Control')=='no-store';assert expected!=200 or r.headers.get('X-Frame-Options')=='DENY';return re.sub('<!--.*?-->','',body,flags=re.S),r.headers,r.url

def form(html,action):return next(f['pairs'] for f in Forms(html).forms if f['action']==action)
def replace(pairs,**changes):return [(k,v) for k,v in pairs if k not in changes]+list(changes.items())
def module(enabled):
 m=next(m for m in f.call(api+'/modules') if m['key']=='helpdesk')
 if m['enabled']!=enabled:f.call(api+'/modules/helpdesk','PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
initial=next(m for m in f.call(api+'/modules') if m['key']=='helpdesk')['enabled']
try:
 module(False);html,_,_=fetch(f.browser,root);assert 'Helpdesk is turned off' in html
 html,_,_=fetch(f.browser,root+'?/module',form(html,'?/module'));assert 'Customer intake' in html
 settings=f.call(api+'/helpdesk/settings')
 if not settings['accepting']:html,_,_=fetch(f.browser,root+'?/configure',form(html,'?/configure'))
 settings=f.call(api+'/helpdesk/settings');url=f.base+'/support/new/'+settings['public_key']
 html,_,_=fetch(customer,url);pairs=form(html,'');code=dict(pairs)['code'];assert len(code)==64
 payload='<img src=x onerror=alert(1)>'
 # Invalid input keeps the message and the same creation code.
 html,_,_=fetch(customer,url,replace(pairs,subject='',requester_name='Customer',body='Keep my draft'),expected=400)
 assert 'Keep my draft' in html and dict(form(html,''))['code']==code
 html,headers,_=fetch(customer,url,replace(form(html,''),subject='Browser support '+payload,requester_name='Web customer',body='Question '+payload))
 assert 'Your ticket is open' in html and '<img src=x' not in html
 path=re.search(r'href="(/support/t/[0-9a-f-]+)"',html)[1];ticket=path.rsplit('/',1)[1];ticket_url=f.base+path
 cookie=next(c for c in jar if c.name=='looski_ticket');assert cookie.path==path and cookie.has_nonstandard_attr('HttpOnly') and cookie.get_nonstandard_attr('SameSite')=='Strict'
 html,_,_=fetch(customer,ticket_url);assert 'Question &lt;img' in html
 html,_,_=fetch(other,ticket_url);assert 'Enter the private access code' in html and 'Question' not in html
 fetch(other,ticket_url+'?/unlock',{'code':'0'*64},expected=404)
 html,_,_=fetch(other,ticket_url+'?/unlock',{'code':code});assert 'Question' in html
 # Staff session and same-origin protections are independent of ticket codes.
 staff_url=root+'/'+ticket;staff,_,_=fetch(f.browser,staff_url);assert 'Internal note' in staff
 fetch(f.browser,staff_url+'?/reply',replace(form(staff,'?/reply'),body='forged',internal='true'),expected=403,origin='https://elsewhere.invalid')
 secret='private-'+key();staff,_,_=fetch(f.browser,staff_url+'?/reply',replace(form(staff,'?/reply'),body=secret,internal='true'));assert secret in staff
 html,_,_=fetch(customer,ticket_url);assert secret not in html and 'Internal note' not in html
 staff,_,_=fetch(f.browser,staff_url+'?/reply',replace(form(staff,'?/reply'),body='Public answer '+payload,internal='false'))
 html,_,_=fetch(customer,ticket_url);assert 'Public answer' in html and '<img src=x' not in html
 fetch(customer,ticket_url+'?/reply',replace(form(html,'?/reply'),body='forged'),expected=403,origin='https://elsewhere.invalid')
 html,_,_=fetch(customer,ticket_url+'?/reply',replace(form(html,'?/reply'),body='Thank you'))
 assert 'Thank you' in html
 staff,_,_=fetch(f.browser,staff_url);details=form(staff,'?/update')
 staff,_,_=fetch(f.browser,staff_url+'?/update',replace(details,status='resolved'))
 html,_,_=fetch(customer,ticket_url);assert 'Sending a reply will reopen' in html
 html,_,_=fetch(customer,ticket_url+'?/reply',replace(form(html,'?/reply'),body='Another question'));assert '<strong>open</strong>' in html
 # Cookie removal and revocation work without changing the staff login.
 html,_,_=fetch(customer,ticket_url+'?/lock',[]);assert 'Enter the private access code' in html
 staff,_,_=fetch(f.browser,staff_url);staff,_,_=fetch(f.browser,staff_url+'?/update',replace(form(staff,'?/update'),revoke='on'))
 fetch(customer,ticket_url+'?/unlock',{'code':code},expected=404)
 html,_,_=fetch(other,ticket_url);assert 'Enter the private access code' in html and 'Another question' not in html
 module(False);fetch(customer,url,expected=404);fetch(f.browser,staff_url,expected=403)
 print('PASS: Helpdesk browser forms, code/cookie isolation, draft retention, public/private messages, CSRF, markup escaping, status/revocation and module gates')
finally:module(initial)

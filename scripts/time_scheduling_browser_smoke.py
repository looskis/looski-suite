#!/usr/bin/env python3
"""Server-rendered browser forms, capability cookies, validation and CSRF for M7/M8."""
import datetime,http.cookiejar,re,secrets,time
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request,build_opener,HTTPCookieProcessor
from forms_browser_html import Forms
from integration_fixture import Fixture
f=Fixture();api='/api/v1/workspaces/'+f.workspace;staff=f.base+'/workspaces/'+f.workspace;key=lambda:secrets.token_hex(16)
jar=http.cookiejar.CookieJar();customer=build_opener(HTTPCookieProcessor(jar));other=build_opener(HTTPCookieProcessor())
def fetch(opener,url,pairs=None,expected=200,origin=None):
 r=Request(url,data=None if pairs is None else urlencode(pairs).encode(),headers={'Accept':'text/html',**({'Origin':origin or f.base} if pairs is not None else {})})
 try:r=opener.open(r,timeout=30)
 except HTTPError as e:r=e
 with r:
  body=r.read().decode();assert r.status==expected,(url,r.status,expected,body[:500]);assert r.headers.get('Cache-Control')=='no-store';return re.sub('<!--.*?-->','',body,flags=re.S),r.url

def form(html,action,index=0):return [f['pairs'] for f in Forms(html).forms if f['action']==action and (action or 'code' in dict(f['pairs']))][index]
def replace(pairs,**changes):return [(k,v) for k,v in pairs if k not in changes]+list(changes.items())
def module(k,enabled):
 m=next(m for m in f.call(api+'/modules') if m['key']==k)
 if m['enabled']!=enabled:f.call(api+'/modules/'+k,'PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
initial={m['key']:m['enabled'] for m in f.call(api+'/modules')}
try:
 module('time',False);html,_=fetch(f.browser,staff+'/time');assert 'Time tracking is turned off' in html
 html,_=fetch(f.browser,staff+'/time?/module',form(html,'?/module'));assert 'Record time' in html
 title='Browser time '+key();html,_=fetch(f.browser,staff+'/time?/save',replace(form(html,'?/save'),description=title,work_on='2026-11-01',minutes='2.25',mode='manual',billable='on'));assert title in html
 html,_=fetch(f.browser,staff+'/time?/budget',replace(form(html,'?/budget'),name='Browser budget',from_on='2026-11-01',through_on='2026-11-30',hours='1'));assert 'Browser budget' in html
 module('scheduling',True);module('forms',True)
 custom=f.call(api+'/forms','POST',{'name':'Browser custom intake','draft':{'fields':[{'key':'detail','label':'What should we prepare?','kind':'long_text','required':True}]}},key=key(),expected=201)
 custom=f.call(api+'/forms/'+custom['id']+'/publish','POST',{'expected_version':custom['version']},key=key())
 html,_=fetch(f.browser,staff+'/scheduling');payload=replace(form(html,'?/save'),name='Browser appointment '+key(),timezone='UTC',form_id=custom['id'],accepting='on',weekly_count='1',day_0='1',start_0='00:00',end_0='23:59')
 # A full-day Monday window allows an anonymous no-JavaScript form journey.
 html,url=fetch(f.browser,staff+'/scheduling?/save',payload);typeid=url.split('type=')[1];t=f.call(api+'/scheduling/types/'+typeid)
 date=datetime.datetime.now(datetime.timezone.utc).date()+datetime.timedelta(days=1)
 while date.isoweekday()!=1:date+=datetime.timedelta(days=1)
 page=f.base+'/book/'+t['public_key']+'?date='+date.isoformat();html,_=fetch(customer,page)
 pairs=form(html,'');code=dict(pairs)['code'];slot=next(s['starts_at'] for s in f.call('/api/v1/public/scheduling/types/'+t['public_key']+'/slots?date='+date.isoformat(),authenticated=False))
 html,_=fetch(customer,page,replace(pairs,starts_at=slot,attendee_name='Browser attendee',attendee_email='browser@example.invalid'),expected=400);assert 'Check the intake' in html and dict(form(html,''))['code']==code
 html,_=fetch(customer,page,replace(form(html,''),starts_at=slot,attendee_name='Browser attendee',attendee_email='browser@example.invalid',**{'answer.detail':'<img src=x onerror=alert(1)>'}));assert 'Your appointment is confirmed' in html
 path=re.search(r'href="(/appointments/[0-9a-f-]+)"',html)[1];booking=path.rsplit('/',1)[1];cookie=next(c for c in jar if c.name=='looski_booking');assert cookie.path==path and cookie.has_nonstandard_attr('HttpOnly') and cookie.get_nonstandard_attr('SameSite')=='Strict'
 html,_=fetch(other,f.base+path);assert 'Private access code' in html
 fetch(other,f.base+path+'?/unlock',{'code':'0'*64},expected=404)
 html,_=fetch(other,f.base+path+'?/unlock',{'code':code});assert 'Cancel appointment' in html
 html,_=fetch(f.browser,staff+'/scheduling/'+booking);assert '&lt;img src=x' in html and '<img src=x' not in html
 fetch(customer,f.base+path+'?/change',{'expected_version':'1','action':'cancel','idempotency_key':key()},expected=403,origin='https://elsewhere.invalid')
 html,_=fetch(customer,f.base+path);cancel=replace(form(html,'?/change',-1),action='cancel')
 html,_=fetch(customer,f.base+path+'?/change',cancel);assert 'cancelled' in html
 html,_=fetch(customer,f.base+path+'?/lock',[]);assert 'Private access code' in html
 print('PASS: browser time entries/budgets, appointment editor, versioned intake validation, private code/cookie isolation, escaping, cancellation and CSRF')
finally:
 for k,v in initial.items():module(k,v)

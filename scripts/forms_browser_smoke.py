#!/usr/bin/env python3
"""Forms through the gateway as a browser without scripting: staff builder, publish,
public and embedded submission, framing headers, CSV, recipe states and session gates."""
import csv
import io
import secrets
import time
from html.parser import HTMLParser
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import HTTPCookieProcessor, HTTPRedirectHandler, Request, build_opener, urlopen
from integration_fixture import Fixture
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}';web=f.base+f'/workspaces/{f.workspace}/forms'
def key():return secrets.token_hex(16)
class Forms(HTMLParser):
 """Successful controls per form, as a browser would send them."""
 def __init__(self,html):
  super().__init__();self.forms=[];self.form=None;self.select=None;self.area=None;self.feed(html)
 def handle_starttag(self,tag,attrs):
  attrs=dict(attrs)
  if tag=='form':self.form={'action':attrs.get('action',''),'pairs':[]};self.forms.append(self.form)
  if self.form is None or 'disabled' in attrs:return
  if tag=='input' and 'name' in attrs:
   if attrs.get('type')!='checkbox' or 'checked' in attrs:self.form['pairs'].append((attrs['name'],attrs.get('value','on' if attrs.get('type')=='checkbox' else '')))
  if tag=='textarea' and 'name' in attrs:self.area=[attrs['name'],''];self.form['pairs'].append(self.area)
  if tag=='select':self.select={'name':attrs['name'],'first':None,'chosen':None}
  if tag=='option' and self.select is not None:
   if self.select['first'] is None:self.select['first']=attrs.get('value','')
   if 'selected' in attrs:self.select['chosen']=attrs.get('value','')
 def handle_data(self,data):
  if self.area is not None:self.area[1]+=data
 def handle_endtag(self,tag):
  if tag=='textarea':self.area=None
  if tag=='select' and self.select is not None:
   chosen=self.select['chosen'] if self.select['chosen'] is not None else self.select['first']
   self.form['pairs'].append((self.select['name'],chosen));self.select=None
  if tag=='form':self.form=None
def form_on(html,action):return next(x for x in Forms(html).forms if x['action']==action)['pairs']
def replace(pairs,**changes):
 """Change named controls; dots in names are written as __."""
 changes={k.replace('__','.'):v for k,v in changes.items()}
 return [(k,v) for k,v in pairs if k not in changes]+[(k,v) for k,v in changes.items() if v is not None]
class Stay(HTTPRedirectHandler):
 def redirect_request(self,*args,**kwargs):return None
anonymous=build_opener(Stay,HTTPCookieProcessor())
def fetch(opener,url,data=None,origin=None,headers=None):
 request=Request(url,data=None if data is None else urlencode(data).encode(),headers={'Accept':'text/html',**({'Origin':origin} if origin else {}),**(headers or {})})
 try:response=opener.open(request,timeout=20)
 except HTTPError as error:response=error
 with response:return response.status,response.read().decode(),response.headers,response.url
def staff(url,data=None,expected=200):
 status,html,headers,final=fetch(f.browser,url,data,f.base if data is not None else None)
 assert status==expected,(url,status,expected,html[:400])
 return html,final
def client():return {'CF-Connecting-IP':'203.0.113.'+str(secrets.randbelow(250)+1)}
def public(url,data=None,origin=None,headers=None):return fetch(anonymous,url,data,origin,headers or client())
def module(name,enabled):
 current=next(m for m in f.call(base+'/modules') if m['key']==name)
 if current['enabled']!=enabled:f.call(base+'/modules/'+name,'PATCH',{'enabled':enabled,'expected_version':current['version']},key=key())
def wait_for(predicate,what,seconds=30):
 deadline=time.monotonic()+seconds
 while True:
  value=predicate()
  if value:return value
  assert time.monotonic()<deadline,'Timed out waiting for '+what
  time.sleep(.2)

# Forms is selected in its own page and works with CRM off.
module('crm',False);module('forms',False)
html,_=staff(web);assert 'Forms is turned off' in html and 'Turn on Forms' in html
html,_=staff(web+'?/module',form_on(html,'?/module'));assert 'Create a form' in html
name='Browser form '+key()
html,url=staff(web+'?/create',replace(form_on(html,'?/create'),name=name,description='Tell us more',confirmation_message='Thanks from the browser test.'))
form_id=url.rsplit('/',1)[-1];path=base+'/forms/'+form_id
form=f.call(path);assert form['name']==name and not form['accepting']
assert 'Publish the form to get its public link' in html
print('PASS: application selection and form creation with CRM off')

def builder(html,op,**changes):
 return staff(url.split('?')[0]+'?/builder',replace(form_on(html,'?/builder'),**changes)+[('op',op)])[0]
html=builder(html,'add',add__label='Full name',add__kind='short_text',add__required='on')
html=builder(html,'add',add__label='Email',add__kind='email')
html=builder(html,'add',add__label='Topic',add__kind='choice',add__options='Sales\r\nSupport')
html=builder(html,'add',add__label='Budget',add__kind='number',add__required='on')
html=builder(html,'add',add__label='Scratch question',add__kind='short_text')
fields=f.call(path)['draft']['fields']
assert [x['key'] for x in fields]==['full_name','email','topic','budget','scratch_question'] and fields[0]['required'] and not fields[1]['required']
assert [o['key'] for o in fields[2]['options']]==['sales','support']
# Edit, reorder, conditions and removal, none of it through raw JSON.
html=builder(html,'save',field__3__show_if='topic=sales',field__0__label='Your full name',field__2__help='Line one\r\nLine two')
fields=f.call(path)['draft']['fields']
assert fields[3]['show_if']=={'field':'topic','equals':['sales']} and fields[0]['label']=='Your full name' and fields[0]['key']=='full_name' and fields[2]['help']=='Line one\nLine two'
html=builder(html,'up:1');assert [x['key'] for x in f.call(path)['draft']['fields']][:2]==['email','full_name']
html=builder(html,'down:0');assert [x['key'] for x in f.call(path)['draft']['fields']][:2]==['full_name','email']
html=builder(html,'remove:4');assert len(f.call(path)['draft']['fields'])==4
html=builder(html,'option_up:2:1');assert [o['key'] for o in f.call(path)['draft']['fields'][2]['options']]==['support','sales']
html=builder(html,'save',field__2__new_options='Partners');assert [o['label'] for o in f.call(path)['draft']['fields'][2]['options']]==['Support','Sales','Partners']
html=builder(html,'remove_option:2:2');assert len(f.call(path)['draft']['fields'][2]['options'])==2
# Rejected changes stay visible, with the draft retained and nothing stored.
before=f.call(path)
rejected,_=staff(url+'?/builder',replace(form_on(html,'?/builder'),field__1__label='Keep my email edit')+[('op','up:3')],400)
assert 'earlier question' in rejected and 'Keep my email edit' in rejected and f.call(path)==before
rejected,_=staff(url+'?/builder',replace(form_on(html,'?/builder'),add__label='Bad key',add__key='Not Valid')+[('op','add')],400)
assert 'lowercase' in rejected and 'Bad key' in rejected and f.call(path)==before
stale=form_on(html,'?/builder')
html=builder(html,'save',description='Tell us more about it')
rejected,_=staff(url+'?/builder',replace(stale,description='My losing draft')+[('op','save')],409)
assert 'My losing draft' in rejected and 'Discard draft and reload current form' in rejected
assert f.call(path)['description']=='Tell us more about it'
print('PASS: builder add, edit, reorder, conditions, options and removal without scripting; rejected drafts retained')

html=builder(html,'publish');assert 'Published.' in html
form=f.call(path);assert form['published_version']['number']==1 and form['accepting']
page=f.base+'/f/'+form['public_key'];embed=page+'/embed'
assert embed in html and '&lt;iframe' in html and page in html
# Framing: only the embed variant may be framed; other routes keep their posture.
for target,framed in [(page,False),(embed,True)]:
 status,body,headers,_=public(target)
 assert status==200 and 'Your full name' in body and 'Tell us more about it' in body
 assert headers.get('Set-Cookie') is None
 assert ('frame-ancestors *' in (headers.get('Content-Security-Policy') or ''))==framed
 assert (headers.get('X-Frame-Options')=='DENY')==(not framed)
for target in [web,f.base+'/login',f.base+f'/workspaces/{f.workspace}/contacts']:
 status,_,headers,_=fetch(f.browser,target)
 assert status==200 and 'frame-ancestors *' not in (headers.get('Content-Security-Policy') or ''),target
print('PASS: public pages reachable without a session; only the embed route is framable')

# The private surface stays behind a session, whatever the public key allows.
for target in [web,url,url+'/versions',url+'/submissions',url+'/submissions/export',url+'/summary',url+'/notifications',url+'/recipe',f.base+f'/workspaces/{f.workspace}/exports']:
 status,_,headers,_=fetch(anonymous,target)
 assert status==303 and headers['Location'].startswith('/login'),(target,status)
status,_,_,_=fetch(anonymous,url+'?/archive',[('expected_version','1'),('archived','true'),('idempotency_key',key())],f.base)
assert status==303 and f.call(path)['archived'] is False
status,body,_,_=public(f.base+f'/f/{"x"*22}');assert status==404 and 'not available' in body
print('PASS: non-public paths require a session')

# A submission from a browser that sends no cookie and has no scripting.
def answers(html,**changes):
 pairs=form_on(html,'')
 return replace(pairs,**{'answer__'+k:v for k,v in changes.items()})
_,body,_,_=public(embed)
missing=answers(body,full_name='Ada Browser',email='ada@example.com',topic='sales',budget='')
status,rejected,_,_=public(embed,missing,f.base);assert status==400 and 'Answer this question' in rejected and 'Ada Browser' in rejected
retry_key=dict(form_on(rejected,''))['submission_key'];assert retry_key==dict(missing)['submission_key']
status,_,_,_=public(embed,answers(body,full_name='Ada Browser',budget='not a number',topic='sales'),f.base);assert status==400
fixed=replace(form_on(rejected,''),answer__budget='5000')
status,done,_,_=public(embed,fixed,f.base);assert status==200 and 'Thanks from the browser test.' in done and 'name="submission_key"' not in done
status,again,_,_=public(embed,fixed,f.base);assert status==200 and 'Thanks from the browser test.' in again
assert f.call(path)['submission_count']==1,'A retried browser submission must not duplicate'
# A cross-origin frame posts from the suite's own origin; only forged origins are refused.
for origin in ['https://elsewhere.example','null']:
 for target in [page,embed]:
  status,_,_,_=public(target,fixed,origin);assert status==403,(target,origin,status)
status,body,_,_=public(page);assert status==200
# Without scripting a hidden question may hold an answer: it is dropped, not stored or rejected.
status,done,_,_=public(page,answers(body,full_name='=HYPERLINK("http://evil.example")',topic='support',budget='99'),f.base);assert status==200
items=f.call(path+'/submissions')['items']
assert items[0]['answers']=={'full_name':'=HYPERLINK("http://evil.example")','topic':'support'} and items[1]['answers']['budget']==5000
for target in ['/login','/workspaces']:
 request=Request(f.base+target,method='HEAD')
 try:headers=urlopen(request,timeout=20).headers
 except HTTPError as failure:headers=failure.headers
 assert headers.get('X-Frame-Options')=='DENY' and "frame-ancestors 'none'" in headers.get('Content-Security-Policy',''),target
print('PASS: cookie-free framed and direct submission, server-side conditions, safe retry, forged origins refused')

# Staff read answers against the version answered, after the draft moves on.
html,_=staff(url);html=builder(html,'publish',field__0__label='Renamed name question')
assert f.call(path)['published_version']['number']==2
status,body,_,_=public(page);assert 'Renamed name question' in body
submission=items[1]['id']
listing,_=staff(url+'/submissions');assert submission in listing and 'Ada Browser' in listing and 'Download CSV' in listing
detail,_=staff(url+'/submissions/'+submission);assert 'Your full name' in detail and 'Renamed name question' not in detail and 'Ada Browser' in detail and 'version 1' in detail
history,_=staff(url+'/versions');assert 'Version 2' in history and 'Version 1' in history and 'Your full name' in history
summary,_=staff(url+'/summary');assert 'Sales' in summary and 'Support' in summary
# Spreadsheet export neutralizes formulas and checks submission access on the server.
status,text,headers,_=fetch(f.browser,url+'/submissions/export');assert status==200 and headers['Content-Type'].startswith('text/csv') and 'attachment' in headers['Content-Disposition']
rows=list(csv.DictReader(io.StringIO(text.lstrip('﻿'))))
assert len(rows)==2 and rows[0]['Renamed name question']=='\'=HYPERLINK("http://evil.example")' and rows[0]['Topic']=='Support' and rows[1]['Budget']=='5000' and rows[1]['version']=='1'
role="(SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner')"
f.sql("DELETE FROM suite.role_permissions WHERE role_id="+role+" AND permission_key='forms.submissions.read'")
try:
 for target in [url+'/submissions/export',url+'/submissions']:
  status,_,_,_=fetch(f.browser,target);assert status==403,target
 assert (status:=fetch(f.browser,url)[0])==200,'Definitions stay readable without answer access'
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"',"+role+",'forms.submissions.read')")
f.sql("DELETE FROM suite.role_permissions WHERE role_id="+role+" AND permission_key='forms.read'")
try:
 status,text,_,_=fetch(f.browser,url+'/submissions/export');assert status==200
 rows=list(csv.DictReader(io.StringIO(text.lstrip('﻿'))));assert 'answers_json' in rows[0] and '5000' in rows[1]['answers_json']
 detail,_=staff(url+'/submissions/'+submission);assert 'full_name' in detail and 'Ada Browser' in detail
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"',"+role+",'forms.read')")
print('PASS: submissions, version-true detail, history, summary and permission-checked CSV with formula neutralization')

# Close, reopen, notifications, export selection.
html,_=staff(url);html,_=staff(url+'?/accepting',form_on(html,'?/accepting'));assert f.call(path)['accepting'] is False
for target in [page,embed]:
 status,body,_,_=public(target);assert status==409 and 'not accepting' in body
status,body,_,_=public(page,fixed,f.base);assert status==409 and 'not accepting' in body
html,_=staff(url+'?/accepting',form_on(html,'?/accepting'));assert f.call(path)['accepting'] is True
owner=f.sql("SELECT m.id FROM suite.memberships m JOIN suite.membership_roles mr ON mr.membership_id=m.id WHERE m.workspace_id='"+f.workspace+"' AND mr.role_id="+role+" LIMIT 1")
notify,_=staff(url+'/notifications')
assert owner in notify
staff(url+'/notifications',form_on(notify,'')+[('membership_ids',owner)])
assert f.call(path)['notification_membership_ids']==[owner]
notify,_=staff(url+'/notifications')
staff(url+'/notifications',[p for p in form_on(notify,'') if p[0]!='membership_ids'])
assert f.call(path)['notification_membership_ids']==[]
for collection,query in [('forms',{}),('form_versions',{'parent_id':form_id}),('form_submissions',{'parent_id':form_id})]:
 preview,_=staff(f.base+f'/workspaces/{f.workspace}/exports?'+urlencode({'collection':collection,'run':'true',**query}));assert 'Download this page' in preview,collection
print('PASS: close/reopen with a clear public message, notification recipients and export selection')

# CRM recipe: mapping, activation, and visible review/failure states with retry.
recipe=url+'/recipe'
off,_=staff(recipe);assert 'CRM choices are unavailable' in off and 'not configured' in off
module('crm',True)
pipeline=f.call(base+'/crm/pipelines','POST',{'name':name},expected=201,key=key())
stage=f.call(base+'/crm/stages','POST',{'name':name,'parent_id':pipeline['id'],'outcome':'open'},expected=201,key=key())
html,_=staff(url);html=builder(html,'publish',add__label='Phone',add__kind='phone')
page_html,_=staff(recipe);assert stage['name'] in page_html
wanted=replace(form_on(page_html,'?/configure'),email_field='email',phone_field='phone',phone_country='US',title_prefix='Browser inquiry',stage=pipeline['id']+'/'+stage['id'])+[('name_fields','full_name')]
rejected,_=staff(recipe+'?/configure',[(k,v) for k,v in wanted if k!='stage'],400);assert 'stage' in rejected
page_html,_=staff(recipe+'?/configure',wanted);assert 'revision 1' in page_html or 'not active' in page_html
stored=f.call(path+'/recipe');assert stored['revision']['mapping']['email_field']=='email' and stored['revision']['mapping']['stage_id']==stage['id']
page_html,_=staff(recipe+'?/activation',form_on(page_html,'?/activation'));assert f.call(path+'/recipe')['active'] and 'active, using mapping revision 1' in page_html
def submit(**values):
 status,body,_,_=public(page);assert status==200,body[:300]
 status,done,_,_=public(page,answers(body,**values),f.base);assert status==200,done[:300]
def latest():return f.call(path+'/submissions?limit=1')['items'][0]['recipe_run']
def settled(states):return wait_for(lambda:(lambda r:r if r and (r['state'] in states or r['job_state'] in ('blocked','exhausted')) else None)(latest()),'recipe run')
email='browser.'+key()[:8]+'@example.com'
submit(full_name='Recipe Browser',email=email,topic='support')
done=settled(('completed',));assert done['state']=='completed'
page_html,_=staff(recipe);assert 'Completed' in page_html and 'Retry</button>' not in page_html and f"crm/opportunities/{done['opportunity_id']}" in page_html
# Failure: a missing permission is visible and retried from the page.
f.sql("DELETE FROM suite.role_permissions WHERE role_id="+role+" AND permission_key='crm.opportunities.create'")
try:
 submit(full_name='Blocked Browser',email='blocked.'+key()[:8]+'@example.com',topic='support')
 run=settled(('completed',));assert run['job_state']=='blocked' and run['error_code']=='authority',run
 page_html,_=staff(recipe);assert 'Failed' in page_html and 'no longer has a permission' in page_html and 'Retry</button>' in page_html
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"',"+role+",'crm.opportunities.create')")
staff(recipe+'?/retry',next(x['pairs'] for x in Forms(page_html).forms if x['action']=='?/retry'))
assert settled(('completed',))['state']=='completed'
# Review: conflicting identifiers wait visibly, then retry finishes.
holder=f.call(base+'/contacts','POST',{'display_name':'Phone holder '+key(),'identifiers':[{'kind':'phone','value':'+1 415 555 0'+str(secrets.randbelow(900)+100),'country':'US'}]},expected=201,key=key())
submit(full_name='Review Browser',email=email,topic='support',phone=holder['identifiers'][0]['value'])
run=settled(('awaiting_review',));assert run['state']=='awaiting_review' and run['intake_id']
page_html,_=staff(recipe);assert 'Needs review' in page_html and f"intakes/{run['intake_id']}" in page_html and 'Retry</button>' in page_html
intake=f.call(base+'/contact-intakes/'+run['intake_id'])
f.call(base+'/contact-intakes/'+run['intake_id']+'/resolve','POST',{'expected_version':intake['version'],'decision':'link','reason':'Same person','contact_id':done['contact_id']},key=key())
staff(recipe+'?/retry',next(x['pairs'] for x in Forms(page_html).forms if x['action']=='?/retry'))
assert settled(('completed',))['state']=='completed'
page_html,_=staff(recipe);page_html,_=staff(recipe+'?/activation',form_on(page_html,'?/activation'));assert not f.call(path+'/recipe')['active']
print('PASS: recipe mapping, activation, visible review and failure states, retry from the web')

# Throttles apply per respondent through the server-rendered page, not per web server.
flood=client();seen=[]
for _ in range(25):
 status,_,_,_=public(page,[('submission_key',key()),('answer.full_name','Flood'),('answer.topic','support')],f.base,flood);seen.append(status)
assert 429 in seen,seen
assert public(page)[0]==200 and public(page,[('submission_key',key()),('answer.full_name','Other client'),('answer.topic','support')],f.base)[0]==200
print('PASS: a throttled respondent does not affect others')

# Archiving through the web closes the public page.
html,_=staff(url);html,_=staff(url+'?/archive',form_on(html,'?/archive'));assert f.call(path)['archived'] is True
assert public(page)[0]==409
html,_=staff(url+'?/archive',form_on(html,'?/archive'));form=f.call(path);assert form['archived'] is False and not form['accepting']
print('PASS: archive and restore from the web')

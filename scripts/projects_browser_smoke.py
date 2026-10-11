#!/usr/bin/env python3
"""Projects through the gateway as a browser without scripting: application selection,
board, moves, scheduling, comments with stored markup staying inert, delivery views,
members, columns, archive, the deal recipe with its visible failure and session gates."""
import re
import secrets
import time
from html import unescape
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import HTTPCookieProcessor, HTTPRedirectHandler, Request, build_opener
from forms_browser_html import Forms
from integration_fixture import Fixture
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}';web=f.base+f'/workspaces/{f.workspace}/projects'
name='Browser project '+secrets.token_hex(4)
def key():return secrets.token_hex(16)
def forms_on(html,action):return [x['pairs'] for x in Forms(html).forms if x['action']==action]
def form_on(html,action,containing=None):
 """The form posting to this action, optionally the one holding a given control value."""
 return next(p for p in forms_on(html,action) if containing is None or any(v==containing for _,v in p))
def replace(pairs,**changes):
 return [(k,v) for k,v in pairs if k not in changes]+[(k,v) for k,v in changes.items() if v is not None]
class Stay(HTTPRedirectHandler):
 def redirect_request(self,*args,**kwargs):return None
anonymous=build_opener(Stay,HTTPCookieProcessor())
def fetch(opener,url,data=None,origin=None):
 request=Request(url,data=None if data is None else urlencode(data).encode(),headers={'Accept':'text/html',**({'Origin':origin} if origin else {})})
 try:response=opener.open(request,timeout=20)
 except HTTPError as error:response=error
 with response:return response.status,response.read().decode(errors='replace'),response.headers,response.url
def staff(url,data=None,expected=200):
 status,html,headers,final=fetch(f.browser,url,data,f.base if data is not None else None)
 assert status==expected,(url,status,expected,html[:600])
 return html,final
def module(name_,enabled):
 current=next(m for m in f.call(base+'/modules') if m['key']==name_)
 if current['enabled']!=enabled:f.call(base+'/modules/'+name_,'PATCH',{'enabled':enabled,'expected_version':current['version']},key=key())
def wait_for(predicate,what,seconds=30):
 deadline=time.monotonic()+seconds
 while True:
  value=predicate()
  if value:return value
  assert time.monotonic()<deadline,'Timed out waiting for '+what
  time.sleep(.2)

# Projects is selected on its own page and works with CRM off.
module('crm',False);module('projects',False)
html,_=staff(web);assert 'Projects is turned off' in html and 'Turn on Projects' in html
html,_=staff(web+'?/module',form_on(html,'?/module'))
assert 'Turn off Projects' in html and 'Create a project' in html
# A rejected creation keeps what was typed and its retry key.
html,_=staff(web)
attempt=replace(form_on(html,'?/create'),name=name,description='Keep this draft',due_on='2031-02-30')
rejected,_=staff(web+'?/create',attempt,expected=400)
assert 'role="alert"' in rejected and 'Keep this draft' in rejected and name in rejected
html,final=staff(web+'?/create',replace(form_on(rejected,'?/create'),due_on='2031-06-30'))
assert name in html and 'To do' in html and 'In progress' in html and 'Done' in html
project=final.rstrip('/').rsplit('/',1)[1];board=web+'/'+project;api=base+'/projects/'+project
assert f.call(api)['name']==name

# Add tasks, including one whose title tries to be markup.
title='<script>alert("title")</script> Fit-out'
html,_=staff(board)
html,_=staff(board+'?/add',replace(form_on(html,'?/add'),title=title,due_on='2031-03-01'))
assert '<script>alert("title")' not in html and '&lt;script>alert(' in html,'Titles are escaped'
html,_=staff(board+'?/add',replace(form_on(html,'?/add'),title='Second task'))
html,_=staff(board+'?/add',replace(form_on(html,'?/add'),title='Third task'))
state=f.call(api+'/board');first,second,third=state['columns'][0]['cards']
assert [first['position'],second['position'],third['position']]==[0,1,2]
existing=f.call(base+'/tasks','POST',{'title':'Existing work '+key()[:6]},expected=201,key=key())
html,_=staff(board)
link=replace(form_on(html,'?/link'),task_id=existing['id'])
html,_=staff(board+'?/link',link)
assert existing['title'] in html and existing['id'] in [c['task']['id'] for column in f.call(api+'/board')['columns'] for c in column['cards']]
# Move buttons: every control posts through the plain form, with no scripting.
html,_=staff(board)
up=next(p for p in forms_on(html,'?/move') if dict(p).get('card_id')==third['id'] and dict(p)['idempotency_key'].endswith(':up'))
html,_=staff(board+'?/move',up)
assert [c['id'] for c in f.call(api+'/board')['columns'][0]['cards']][:3]==[first['id'],third['id'],second['id']]
down=next(p for p in forms_on(html,'?/move') if dict(p).get('card_id')==first['id'] and dict(p)['idempotency_key'].endswith(':down'))
html,_=staff(board+'?/move',down)
assert [c['id'] for c in f.call(api+'/board')['columns'][0]['cards']][:3]==[third['id'],first['id'],second['id']]
done=state['columns'][2]['id']
to_column=next(p for p in forms_on(html,'?/move') if dict(p).get('card_id')==second['id'] and dict(p)['idempotency_key'].endswith(':column'))
staged=replace(to_column,column_id=done)
html,_=staff(board+'?/move',staged)
moved=f.call(api+'/cards/'+second['id'])
assert moved['column_id']==done and moved['task']['status']=='completed'
# A stale page shows the reason rather than undoing the newer move.
rejected,_=staff(board+'?/move',replace(staged,idempotency_key=key()),expected=409)
assert 'role="alert"' in rejected and 'reload the board' in rejected
# Forged origins are refused for every action.
for action in ['?/add','?/move','?/link']:
 assert fetch(f.browser,board+action,staged,'https://evil.example')[0]==403
print('PASS: application selection, draft-preserving creation, escaped titles, add, link and scripting-free moves')

# Card page: assignment, deadline and comments, with stored markup staying inert.
card=board+'/cards/'+first['id']
html,_=staff(card)
plan=replace(form_on(html,'?/schedule'),due_on='2031-04-01')
html,_=staff(card+'?/schedule',plan)
assert f.call(api+'/cards/'+first['id'])['task']['due_on']=='2031-04-01'
payload='<img src=x onerror=alert(1)> **strong words** <script>alert(2)</script> [site](https://example.com/page)'
unsafe=replace(form_on(html,'?/comment'),body='Please keep me [bad](javascript:alert(1))')
rejected,_=staff(card+'?/comment',unsafe,expected=400)
assert 'role="alert"' in rejected and 'Please keep me' in rejected,'A refused comment keeps its draft'
html,_=staff(card+'?/comment',replace(form_on(rejected,'?/comment'),body=payload))
# Outside the edit box, which holds the escaped source, there is no live markup.
stripped=re.sub(r'<textarea.*?</textarea>','',html,flags=re.S)
assert '<img src=x' not in stripped and '<script>alert(2)' not in stripped and 'onerror=alert' in unescape(stripped)
assert '<strong>strong words</strong>' in stripped
assert re.search(r'<a [^>]*href="https://example.com/page"[^>]*rel="noopener noreferrer nofollow"|<a [^>]*rel="noopener noreferrer nofollow"[^>]*href="https://example.com/page"',stripped)
comment=f.call(api+'/cards/'+first['id']+'/comments')['items'][0]
# Edit keeps the source; a rejected edit stays open with what was typed.
edit=form_on(html,'?/edit',comment['id'])
rejected,_=staff(card+'?/edit',replace(edit,body='Edited [bad](data:text/html,x)'),expected=400)
assert 'Edited [bad]' in rejected
html,_=staff(card+'?/edit',replace(edit,body='Edited **text**'))
assert '<strong>text</strong>' in html and 'edited' in html
remove=form_on(html,'?/remove',comment['id'])
html,_=staff(card+'?/remove',remove)
assert 'This comment was removed.' in html and 'Edited' not in re.sub(r'<small.*?</small>','',html,flags=re.S).split('Comments</h2>')[1].split('<section')[0].replace('edited','')
assert fetch(anonymous,card)[0] in (302,303)
print('PASS: scheduling, inert comment markup, refused unsafe links keep drafts, edit and removal')

# Delivery and my work.
delivery,_=staff(board+'/delivery')
assert 'Tasks by deadline' in delivery and 'Workload' in delivery and 'Second task' in delivery
filtered,_=staff(board+'/delivery?status=completed')
assert 'Second task' in filtered and 'Third task' not in filtered
filtered,_=staff(board+'/delivery?due=unscheduled')
assert 'Third task' in filtered and 'Second task' not in filtered
mine,_=staff(web+'/my-work')
assert 'My work' in mine
# Members: add through the directory, remove, with no identifiers typed.
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Browser teammate') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
html,_=staff(board+'/members')
assert 'Browser teammate' in html
html,_=staff(board+'/members?/add',replace(form_on(html,'?/add'),membership_id=other))
assert other in [m['id'] for m in f.call(api+'/members')]
html,_=staff(board+'/members?/remove',form_on(html,'?/remove',other))
assert other not in [m['id'] for m in f.call(api+'/members')]
# Settings: project details, columns and archive.
html,_=staff(board+'/settings')
html,_=staff(board+'/settings?/save',replace(form_on(html,'?/save'),description='Updated description',status='on_hold'))
assert f.call(api)['status']=='on_hold' and f.call(api)['description']=='Updated description'
html,_=staff(board+'/settings?/addColumn',replace(form_on(html,'?/addColumn'),name='Review',task_status=''))
columns=f.call(api+'/columns');review=next(c for c in columns if c['name']=='Review')
assert review['task_status'] is None
html,_=staff(board+'/settings?/saveColumn',replace(form_on(html,'?/saveColumn',review['id']),name='QA review',position='1'))
assert f.call(api+'/columns')[0]['name']=='QA review'
to_do=next(c for c in f.call(api+'/columns') if c['name']=='To do')
html,_=staff(board+'/settings?/removeColumn',replace(form_on(html,'?/removeColumn',review['id']),move_cards_to=to_do['id']))
assert 'QA review' not in [c['name'] for c in f.call(api+'/columns')]
html,_=staff(board+'/settings?/archive',form_on(html,'?/archive'))
assert 'Restore project' in html and f.call(api)['archived']
locked,_=staff(board);assert not forms_on(locked,'?/move') and not forms_on(locked,'?/add')
lockedcard,_=staff(card);assert not forms_on(lockedcard,'?/comment')
html,_=staff(board+'/settings');html,_=staff(board+'/settings?/archive',form_on(html,'?/archive'))
assert not f.call(api)['archived']
print('PASS: delivery filters, members, column management and archive')

# The recipe through its pages, including a failure anyone looking can see and retry.
html,_=staff(web+'/recipes')
assert 'Choosing a milestone needs CRM' in html
module('crm',True)
sales='Browser sales '+key()[:8]
category=f.call(base+'/crm/categories','POST',{'name':sales},expected=201,key=key())
planning=f.call(base+'/crm/statuses','POST',{'name':sales+' planning','parent_id':category['id']},expected=201,key=key())
won=f.call(base+'/crm/statuses','POST',{'name':sales+' won','parent_id':category['id']},expected=201,key=key())
html,_=staff(web+'/recipes')
assert sales+' won' in html
attempt=replace(form_on(html,'?/create'),name=name,status_id=won['id'],name_prefix='x'*80+'y')
rejected,_=staff(web+'/recipes?/create',attempt,expected=400)
assert 'role="alert"' in rejected
html,final=staff(web+'/recipes?/create',replace(form_on(rejected,'?/create'),name_prefix='Delivery',status_id=won['id']))
recipe_id=final.rstrip('/').rsplit('/',1)[1];recipe=web+'/recipes/'+recipe_id;recipe_api=base+'/projects/recipes/'+recipe_id
assert f.call(recipe_api)['revision']['configuration']['name_prefix']=='Delivery'
assert 'Activate recipe' in html
html,_=staff(recipe+'?/active',form_on(html,'?/active'))
assert 'Deactivate recipe' in html
deal=f.call(base+'/crm/deals','POST',{'title':'Browser deal '+key()[:6],'category_id':category['id'],'status_id':planning['id']},expected=201,key=key())
current=f.call(base+'/crm/deals/'+deal['id'])
f.call(base+'/crm/deals/'+deal['id'],'PATCH',{'title':current['title'],'category_id':category['id'],'status_id':won['id'],'expected_version':current['version']},key=key())
run=wait_for(lambda:next((r for r in f.call(recipe_api+'/runs')['items'] if r['state']=='completed'),None),'conversion')
html,_=staff(recipe)
assert 'Project created' in html and f'/projects/{run["project_id"]}' in html and f'/crm/deals/{deal["id"]}' in html
made,_=staff(web+'/'+run['project_id']);assert 'Delivery: '+deal['title'] in made and 'Created from' in made
# A blocked run says why and retries through its button.
module('projects',False)
other_deal=f.call(base+'/crm/deals','POST',{'title':'Blocked deal '+key()[:6],'category_id':category['id'],'status_id':won['id']},expected=201,key=key())
blocked_state=wait_for(lambda:f.sql("SELECT j.state FROM projects.deal_conversions c JOIN suite.jobs j ON j.id=c.job_id WHERE c.deal_id='"+other_deal['id']+"'")=='blocked','blocked run')
module('projects',True)
html,_=staff(recipe)
assert 'Failed' in html and 'CRM or Projects was turned off' in html
run=next(r for r in f.call(recipe_api+'/runs')['items'] if r['deal_id']==other_deal['id'])
html,_=staff(recipe+'?/retry',form_on(html,'?/retry',run['id']))
wait_for(lambda:next(r for r in f.call(recipe_api+'/runs')['items'] if r['id']==run['id'])['state']=='completed','retried run')
job,_=staff(f.base+f'/workspaces/{f.workspace}/jobs/'+run['job_id']);assert 'Deal-to-project conversion' in job
print('PASS: recipe configuration, activation, runs, visible failure and retry through pages')

# Session gates and framing, on every page.
pages=[web,web+'/my-work',web+'/recipes',recipe,board,board+'/delivery',board+'/members',board+'/settings',card]
for page in pages:
 status,_,headers,_=fetch(anonymous,page)
 assert status in (302,303) and 'login' in headers['Location'],(page,status)
 status,_,headers,_=fetch(f.browser,page)
 assert status==200 and "frame-ancestors 'none'" in headers['Content-Security-Policy'] and headers['X-Frame-Options']=='DENY' and headers['Referrer-Policy']=='same-origin',(page,dict(headers))
assert fetch(f.browser,web+'/'+secrets.token_hex(16))[0]==404
# Disabled: pages close and say so; data remains.
module('projects',False)
html,_=staff(web);assert 'Projects is turned off' in html
assert fetch(f.browser,board)[0]==403
task_page,_=staff(f.base+f'/workspaces/{f.workspace}/tasks/'+first['task']['id']);assert 'Fit-out' in unescape(task_page)
module('projects',True);module('crm',True)
print('PASS: session gates, framing headers, not-found and disabled pages')
print('PASS: Projects browser journey')

#!/usr/bin/env python3
"""Projects: grouping, membership, boards, delivery views and comments over shared Tasks,
and the deal-to-project recipe (Milestone 4 acceptance), with real second users."""
import base64
import hashlib
import hmac
import json
import secrets
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor
from dev import ROOT
from integration_fixture import Fixture
f=Fixture();call,sql=f.call,f.sql
base='/api/v1/workspaces/'+f.workspace;projects=base+'/projects'
name='Projects '+secrets.token_hex(5)
def key():return secrets.token_hex(16)
def module(k,enabled):
 m=next(m for m in call(base+'/modules') if m['key']==k)
 if m['enabled']!=enabled:m=call(base+'/modules/'+k,'PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
 return m
def wait_for(predicate,what,seconds=30):
 deadline=time.monotonic()+seconds
 while True:
  value=predicate()
  if value:return value
  assert time.monotonic()<deadline,'Timed out waiting for '+what
  time.sleep(.2)
def auth(path,body,token=None):
 payload={'path':path,'body':body,'token':token}
 script="const x=JSON.parse(require('fs').readFileSync(0,'utf8')); fetch('http://auth:9999'+x.path,{method:'POST',headers:{'Content-Type':'application/json',...(x.token?{Authorization:'Bearer '+x.token}:{})},body:JSON.stringify(x.body)}).then(async r=>{if(!r.ok){console.error(r.status);process.exit(1)} console.log(await r.text())})"
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
  email='projects-'+key()+'@example.invalid';password=secrets.token_urlsafe(24)
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
def add(project,title,**task):return call(projects+'/'+project+'/cards','POST',{'task':{'title':title,**task}},expected=201,key=key())
def board(project,person=None):return (person.call if person else call)(projects+'/'+project+'/board')
def density(project):
 rows=sql("SELECT column_id,count(*),count(DISTINCT position),min(position),max(position) FROM projects.cards WHERE project_id='"+project+"' AND removed_at IS NULL GROUP BY column_id")
 for row in filter(None,rows.splitlines()):
  _,count,distinct,low,high=row.split('|')
  assert count==distinct and low=='0' and int(high)==int(count)-1,row

# Projects works with CRM off; disabled closes direct operations and the column list.
module('crm',False);module('projects',False)
call(projects,expected=403)
call(projects,'POST',{'name':name},expected=403,key=key())
module('projects',True)
call(projects,'POST',{'name':' '},expected=400,key=key())
call(projects,'POST',{'name':name,'due_on':'2026-02-30'},expected=400,key=key())
call(projects,'POST',{'name':name},expected=400)
retry=key();body={'name':name,'description':'Delivery work','due_on':'2031-06-30'}
project=call(projects,'POST',body,expected=201,key=retry)
assert call(projects,'POST',body,expected=201,key=retry)==project,'A retried creation returns the original'
call(projects,'POST',{'name':name+' other'},expected=409,key=retry)
assert sql("SELECT count(*) FROM projects.projects WHERE workspace_id='"+f.workspace+"' AND name='"+name+"'")=='1'
assert project['is_member'] and project['status']=='active' and project['source_deal_id'] is None
path=projects+'/'+project['id']
owner_membership=sql("SELECT created_by_membership_id FROM projects.projects WHERE id='"+project['id']+"'")
assert [m['id'] for m in call(path+'/members')]==[owner_membership],'The creator joins'
assert call(projects+'?search='+name.replace(' ','%20'))['items'][0]['id']==project['id']
edited=call(path,'PATCH',{**body,'status':'on_hold','expected_version':project['version']},key=key())
call(path,'PATCH',{**body,'expected_version':project['version']},expected=409,key=key())
project=call(path,'PATCH',{**body,'expected_version':edited['version']},key=key())
assert sql("SELECT count(*) FROM suite.audit_events WHERE record_id='"+project['id']+"' AND action IN ('projects.created','projects.saved')")=='3'
print('PASS: projects without CRM, disabled refusal, validation, retries, stale versions, audit')

# Tasks stays the single source of task state; no project table copies it.
assert sql("SELECT count(*) FROM information_schema.columns WHERE table_schema='projects' AND table_name IN ('cards') AND column_name IN ('title','notes','assigned_membership_id','due_on','status')")=='0'
assert sql("SELECT count(*) FROM information_schema.tables WHERE table_schema='projects' AND table_name LIKE '%task%'")=='0'
columns=call(path+'/columns')
assert [(c['name'],c['task_status']) for c in columns]==[('To do','open'),('In progress','in_progress'),('Done','completed')]
todo,doing,done=[c['id'] for c in columns]
card=add(project['id'],'First task',due_on='2031-07-15')
assert card['column_id']==todo and card['position']==0 and card['task']['status']=='open' and card['assignee_in_project']
assert call(base+'/tasks/'+card['task']['id'])==card['task'],'The card holds the shared task'
assert call(path+'/cards/'+card['id'])==card
external=call(base+'/tasks','POST',{'title':'Existing task','status':'completed'},expected=201,key=key())
linked=call(path+'/cards','POST',{'task_id':external['id']},expected=201,key=key())
assert linked['column_id']==done and linked['task']==external,'Linking leaves the task alone and files it by status'
call(path+'/cards','POST',{'task_id':external['id']},expected=409,key=key())
call(path+'/cards','POST',{'task_id':external['id'],'task':{'title':'x'}},expected=400,key=key())
call(path+'/cards','POST',{},expected=400,key=key())
other_project=call(projects,'POST',{'name':name+' second'},expected=201,key=key())
call(projects+'/'+other_project['id']+'/cards','POST',{'task_id':external['id']},expected=409,key=key())
# A status-mapped column asks Tasks for the change; the shared history records it.
moved=call(path+'/cards/'+card['id']+'/move','POST',{'expected_version':card['version'],'column_id':done,'before_card_id':None},key=key())
assert moved['column_id']==done and moved['position']==1 and moved['task']['status']=='completed' and moved['task']['completed_at']
assert [c['snapshot']['status'] for c in call(base+'/tasks/'+card['task']['id']+'/history')['items']]==['open','completed']
call(path+'/cards/'+card['id']+'/move','POST',{'expected_version':card['version'],'column_id':todo,'before_card_id':None},expected=409,key=key())
call(path+'/cards/'+card['id']+'/move','POST',{'expected_version':moved['version'],'column_id':todo,'before_card_id':card['id']},expected=400,key=key())
retry=key();change={'expected_version':moved['version'],'column_id':doing,'before_card_id':None}
back=call(path+'/cards/'+card['id']+'/move','POST',change,key=retry)
assert call(path+'/cards/'+card['id']+'/move','POST',change,key=retry)==back and back['task']['status']=='in_progress' and back['task']['completed_at'] is None
# Assignment and deadline through Tasks, under the project's own assignment rule.
outsider=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Projects outsider') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
schedule=path+'/cards/'+card['id']+'/schedule'
call(schedule,'POST',{'expected_task_version':back['task']['version'],'assigned_membership_id':outsider,'due_on':None},expected=400,key=key())
call(schedule,'POST',{'expected_task_version':1,'assigned_membership_id':owner_membership,'due_on':None},expected=409,key=key())
call(schedule,'POST',{'expected_task_version':back['task']['version'],'assigned_membership_id':owner_membership,'due_on':'2031-13-01'},expected=400,key=key())
call(path+'/cards','POST',{'task':{'title':'Outsider work','assigned_membership_id':outsider}},expected=400,key=key())
scheduled=call(schedule,'POST',{'expected_task_version':back['task']['version'],'assigned_membership_id':owner_membership,'due_on':'2031-08-01','status':'open'},key=key())
assert scheduled['task']['due_on']=='2031-08-01' and scheduled['task']['status']=='open'
assert scheduled['task']['title']=='First task','Scheduling keeps the rest of the task'
# Tasks may assign outside the project; the board flags rather than hides it.
current=scheduled['task']
reassigned=call(base+'/tasks/'+current['id'],'PATCH',{'title':current['title'],'notes':current['notes'],'assigned_membership_id':outsider,'due_on':current['due_on'],'status':current['status'],'expected_version':current['version']},key=key())
flagged=call(path+'/cards/'+card['id']);assert not flagged['assignee_in_project'] and flagged['assignee_name'] is None
call(schedule,'POST',{'expected_task_version':reassigned['version'],'assigned_membership_id':owner_membership,'due_on':'2031-08-01'},key=key())
# Removing keeps the task; adding it back reuses the card and its discussion.
removed_card=add(project['id'],'Temporary')
gone=call(path+'/cards/'+removed_card['id']+'/remove','POST',{'expected_version':removed_card['version']},key=key())
assert gone['column_id'] is None and gone['position'] is None
assert call(base+'/tasks/'+removed_card['task']['id'])['title']=='Temporary'
assert removed_card['id'] in [c['id'] for c in call(path+'/cards?ended=true')['items']]
restored=call(path+'/cards','POST',{'task_id':removed_card['task']['id']},expected=201,key=key())
assert restored['id']==removed_card['id']
density(project['id'])
print('PASS: cards over shared Tasks, status-mapped moves, scheduling rules, removal and re-adding')

# Columns: rename, reorder, remap and removal with an explicit destination.
extra=call(path+'/columns','POST',{'name':'Review','task_status':None,'position':1},expected=201,key=key())
assert [c['name'] for c in call(path+'/columns')]==['To do','Review','In progress','Done']
call(path+'/columns','POST',{'name':'','task_status':None},expected=400,key=key())
extra=call(path+'/columns/'+extra['id'],'PATCH',{'name':'QA','task_status':None,'position':3,'expected_version':extra['version']},key=key())
assert [c['name'] for c in call(path+'/columns')]==['To do','In progress','Done','QA']
call(path+'/columns/'+extra['id'],'PATCH',{'name':'QA','task_status':None,'expected_version':extra['version']-1},expected=409,key=key())
parked=add(project['id'],'Parked')
parked=call(path+'/cards/'+parked['id']+'/move','POST',{'expected_version':parked['version'],'column_id':extra['id'],'before_card_id':None},key=key())
assert parked['task']['status']=='open','An unmapped column leaves the status alone'
call(path+'/columns/'+extra['id']+'/remove','POST',{'expected_version':extra['version'],'move_cards_to':extra['id']},expected=400,key=key())
call(path+'/columns/'+extra['id']+'/remove','POST',{'expected_version':extra['version'],'move_cards_to':secrets.token_hex(16)},expected=404,key=key())
call(path+'/columns/'+extra['id']+'/remove','POST',{'expected_version':extra['version'],'move_cards_to':done},key=key())
after=call(path+'/cards/'+parked['id']);assert after['column_id']==done and after['task']['status']=='open'
assert [c['name'] for c in call(path+'/columns')]==['To do','In progress','Done'] and [c['position'] for c in call(path+'/columns')]==[0,1,2]
density(project['id'])
print('PASS: column rename, reorder, remap and removal')

# Concurrent board moves: nobody's place is lost and nobody shares one.
race=call(projects,'POST',{'name':name+' race'},expected=201,key=key());race_path=projects+'/'+race['id']
first,second,_=[c['id'] for c in call(race_path+'/columns')]
racers=[add(race['id'],'Racer '+str(i)) for i in range(12)]
def move(card,target,before=None,expected=(200,409),k=None):
 from urllib.error import HTTPError
 from urllib.request import Request,urlopen
 request=Request(f.base+race_path+'/cards/'+card['id']+'/move',method='POST',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':k or key()},data=json.dumps({'expected_version':card['version'],'column_id':target,'before_card_id':before}).encode())
 try:response=urlopen(request,timeout=30)
 except HTTPError as failure:response=failure
 with response:
  assert response.status in expected,response.status
  return response.status,json.loads(response.read())
head=racers[0]['id']
with ThreadPoolExecutor(max_workers=12) as pool:
 # Half race to the same place in another column; half reorder inside the source.
 results=list(pool.map(lambda c:move(c,second) if racers.index(c)%2 else move(c,first,head if c['id']!=head else None),racers))
assert all(status==200 for status,_ in results),[s for s,_ in results]
density(race['id'])
state=board(race['id'])
placed=[c['id'] for column in state['columns'] for c in column['cards']]
assert sorted(placed)==sorted(c['id'] for c in racers) and len(set(placed))==12,'Every card is on the board exactly once'
for column in state['columns']:assert [c['position'] for c in column['cards']]==list(range(len(column['cards'])))
assert len(state['columns'][1]['cards'])==6
# The same card raced to different places: exactly one wins, the others are told.
target=call(race_path+'/cards/'+racers[1]['id'])
with ThreadPoolExecutor(max_workers=8) as pool:
 outcomes=list(pool.map(lambda i:move(target,first if i%2 else second),range(8)))
assert sum(1 for status,_ in outcomes if status==200)==1 and sum(1 for status,_ in outcomes if status==409)==7,[s for s,_ in outcomes]
density(race['id'])
# The same retry raced: one effect, the same answer.
target=call(race_path+'/cards/'+racers[2]['id']);shared=key()
before=int(sql("SELECT count(*) FROM suite.audit_events WHERE record_id='"+target['id']+"' AND action='projects.card.moved'"))
with ThreadPoolExecutor(max_workers=6) as pool:
 outcomes=list(pool.map(lambda _:move(target,second,k=shared),range(6)))
assert all(status==200 for status,_ in outcomes) and len({json.dumps(body,sort_keys=True) for _,body in outcomes})==1
assert int(sql("SELECT count(*) FROM suite.audit_events WHERE record_id='"+target['id']+"' AND action='projects.card.moved'"))==before+1
density(race['id'])
# The database refuses a shared place even if application code ever did not.
try:
 sql("UPDATE projects.cards SET position=(SELECT min(position) FROM projects.cards WHERE column_id='"+second+"') WHERE column_id='"+second+"'")
 raise AssertionError('Duplicate position accepted')
except subprocess.CalledProcessError:pass
print('PASS: concurrent moves keep dense unique places; stale moves conflict; retries apply once; database constraint holds')

# Project scope is membership, checked live with the same token.
reader=Person('Projects reader',['projects.read:assigned','tasks.read'])
reader.call(projects+'/'+project['id'],expected=404)
reader.call(path+'/board',expected=404)
assert project['id'] not in [p['id'] for p in reader.call(projects)['items']]
reader.call(path+'/members','POST',{'membership_id':reader.membership},expected=403)
call(path+'/members','POST',{'membership_id':secrets.token_hex(16)},expected=400,key=key())
retry=key();member=call(path+'/members','POST',{'membership_id':reader.membership},expected=201,key=retry)
assert member['display_name']=='Projects reader' and call(path+'/members','POST',{'membership_id':reader.membership},expected=201,key=retry)==member
call(path+'/members','POST',{'membership_id':reader.membership},expected=409,key=key())
seen=reader.call(path);assert seen['is_member']
assert reader.call(projects)['items'][0]['id']==project['id']
assert reader.call(path+'/board')['columns'][0]['cards'] is not None
reader.call(path,'PATCH',{**body,'expected_version':project['version']},expected=403)
reader.call(path+'/columns','POST',{'name':'x','task_status':None},expected=403)
reader.call(path+'/cards','POST',{'task':{'title':'x'}},expected=403)
reader.call(projects+'/recipes',expected=403)
reader.call(projects+'/directory',expected=403)
reader.call(base+'/crm/deals',expected=403)
call(path+'/members/'+reader.membership+'/remove','POST',expected=200,key=key())
reader.call(path,expected=404)
reader.call(path+'/cards/'+card['id'],expected=404)
call(path+'/members/'+reader.membership+'/remove','POST',expected=404,key=key())
call(path+'/members','POST',{'membership_id':reader.membership},expected=201,key=key())
# Having no Projects grant at all outweighs membership.
nothing=Person('Projects nobody',['tasks.read'])
call(path+'/members','POST',{'membership_id':nothing.membership},expected=201,key=key())
nothing.call(projects,expected=403);nothing.call(path,expected=403);nothing.call(path+'/board',expected=403)
call(path+'/members/'+nothing.membership+'/remove','POST',key=key())
# An assigned editor can change their own projects only.
editor=Person('Projects editor',['projects.read:assigned','projects.manage:assigned','projects.board.edit:assigned','projects.members.manage:assigned','projects.create:assigned','tasks.read','tasks.create','tasks.update'])
editor.call(path+'/cards','POST',{'task':{'title':'x'}},expected=404)
mine=editor.call(projects,'POST',{'name':name+' editor'},expected=201)
assert mine['is_member'] and editor.call(projects+'/'+mine['id']+'/board')
print('PASS: membership as project scope, live revocation, grant still required, assigned editors')

# Task visibility is Tasks' decision, not the project's.
blind=Person('Projects blind',['projects.read'])
state=board(project['id'],blind)
assert all(column['cards']==[] for column in state['columns']),'No task read means no cards'
blind.call(path+'/cards/'+card['id'],expected=404);blind.call(path+'/cards/'+card['id']+'/comments',expected=404)
report=blind.call(path+'/delivery');assert report['summary']['total']==0 and report['items']==[] and report['workload']==[]
assert blind.call(projects+'/my-work')['items']==[]
assert blind.call(path+'/cards')['items']==[]
own=Person('Projects own tasks',['projects.read','projects.board.edit','projects.comments.write','tasks.read:assigned','tasks.update:assigned','tasks.create:assigned'])
call(path+'/members','POST',{'membership_id':own.membership},expected=201,key=key())
hers=add(project['id'],'Her task',assigned_membership_id=own.membership)
everyone=[c for column in board(project['id'])['columns'] for c in column['cards']]
assert len(everyone)>3
visible=[c for column in board(project['id'],own)['columns'] for c in column['cards']]
assert [c['id'] for c in visible]==[hers['id']] and visible[0]['position']==0,'A place counts only what the member can see'
assert hers['position']>0,'Hidden cards sit ahead of hers in the full board'
assert own.call(path+'/delivery')['summary']['total']==1
hidden=everyone[0]
own.call(path+'/cards/'+hidden['id'],expected=404)
own.call(path+'/cards/'+hidden['id']+'/comments','POST',{'body':'hello'},expected=404)
own.call(path+'/cards/'+hidden['id']+'/move','POST',{'expected_version':hidden['version'],'column_id':doing,'before_card_id':None},expected=404)
own.call(path+'/cards/'+hidden['id']+'/schedule','POST',{'expected_task_version':hidden['task']['version'],'assigned_membership_id':own.membership,'due_on':None},expected=404)
own.call(path+'/cards/'+hidden['id']+'/remove','POST',{'expected_version':hidden['version']},expected=404)
own.call(path+'/cards/'+hers['id']+'/move','POST',{'expected_version':hers['version'],'column_id':hers['column_id'],'before_card_id':hidden['id']},expected=404)
own.call(path+'/cards','POST',{'task_id':external['id']},expected=404)
# Editing a board does not extend Tasks authority: a mapped move rolls back whole.
frozen=Person('Projects frozen',['projects.read','projects.board.edit','tasks.read'])
held=add(project['id'],'Frozen task')
places=sql("SELECT column_id,position FROM projects.cards WHERE id='"+held['id']+"'")
frozen.call(path+'/cards/'+held['id']+'/move','POST',{'expected_version':held['version'],'column_id':done,'before_card_id':None},expected=403)
assert sql("SELECT column_id,position FROM projects.cards WHERE id='"+held['id']+"'")==places and call(path+'/cards/'+held['id'])==held
frozen.call(path+'/cards/'+held['id']+'/move','POST',{'expected_version':held['version'],'column_id':todo,'before_card_id':None})
density(project['id'])
print('PASS: tasks hidden independently, no hidden IDs or counts, Tasks authority kept on moves')

# Delivery views: deadlines, workload and my work, over what the member can see.
today=sql("SELECT current_date::text");soon=sql("SELECT (current_date+3)::text");past=sql("SELECT (current_date-3)::text")
delivery_project=call(projects,'POST',{'name':name+' delivery','due_on':soon},expected=201,key=key());dp=projects+'/'+delivery_project['id']
late=add(delivery_project['id'],'Late',due_on=past);near=add(delivery_project['id'],'Near',due_on=soon)
beyond=add(delivery_project['id'],'Beyond the project',due_on='2040-01-01');loose=add(delivery_project['id'],'No date')
finished=add(delivery_project['id'],'Finished long ago',due_on=past,status='completed')
report=call(dp+'/delivery')
assert report['today']==today
summary=report['summary']
assert (summary['total'],summary['open'],summary['completed'],summary['overdue'],summary['due_soon'],summary['unscheduled'],summary['past_project_deadline'])==(5,4,1,1,1,1,1),summary
assert [i['task']['title'] for i in report['items']][:2] in (['Late','Finished long ago'],['Finished long ago','Late']) and report['items'][-1]['task']['title']=='No date'
assert next(i for i in report['items'] if i['task']['title']=='Finished long ago')['overdue'] is False
assert [i['task']['title'] for i in call(dp+'/delivery?due=overdue')['items']]==['Late']
assert [i['task']['title'] for i in call(dp+'/delivery?due=soon')['items']]==['Near']
assert [i['task']['title'] for i in call(dp+'/delivery?due=unscheduled')['items']]==['No date']
assert [i['task']['title'] for i in call(dp+'/delivery?status=completed')['items']]==['Finished long ago']
assert call(dp+'/delivery?assigned_membership_id='+outsider)['items']==[] and call(dp+'/delivery?due=overdue')['summary']['total']==5
assert [(l['membership_id'],l['unfinished'],l['overdue']) for l in report['workload']]==[(owner_membership,4,1)]
def all_work(extra=''):
 items,after=[],None
 while True:
  page=call(projects+'/my-work?limit=100'+extra+('&after='+after if after else ''))
  items+=page['items'];after=page['next_cursor']
  if not after:return items
work={'items':all_work()}
assert late['id'] in [i['card_id'] for i in work['items']] and finished['id'] not in [i['card_id'] for i in work['items']]
assert next(i for i in work['items'] if i['card_id']==late['id'])['overdue'] and next(i for i in work['items'] if i['card_id']==late['id'])['project_name']==name+' delivery'
assert finished['id'] in [i['card_id'] for i in all_work('&include_finished=true')]
assert [i['card_id'] for i in own.call(projects+'/my-work')['items']]==[hers['id']]
page=call(projects+'/my-work?limit=1');assert len(page['items'])==1 and page['next_cursor']
following=call(projects+'/my-work?limit=1&after='+page['next_cursor']);assert following['items'][0]['card_id']!=page['items'][0]['card_id']
call(base+'/tasks/'+near['task']['id']+'/archive','POST',{'expected_version':near['task']['version'],'archived':True},key=key())
assert call(dp+'/delivery')['summary']['total']==4 and near['id'] not in [c['id'] for column in board(delivery_project['id'])['columns'] for c in column['cards']],'An archived task leaves the views'
call(dp+'/cards/'+near['id']+'/move','POST',{'expected_version':near['version'],'column_id':call(dp+'/columns')[1]['id'],'before_card_id':None},expected=409,key=key())
print('PASS: delivery summary, deadlines, filters, workload, my work, archived tasks')

# Comments are stored as source and only ever leave as typed text.
comments=path+'/cards/'+card['id']+'/comments'
payload='<img src=x onerror=alert(1)> **bold** <script>alert(2)</script> [site](https://example.com/a)'
call(comments,'POST',{'body':'[bad](javascript:alert(1))'},expected=400,key=key())
call(comments,'POST',{'body':'  '},expected=400,key=key())
call(comments,'POST',{'body':'x'*10001},expected=400,key=key())
retry=key();comment=call(comments,'POST',{'body':payload},expected=201,key=retry)
assert call(comments,'POST',{'body':payload},expected=201,key=retry)==comment
assert comment['body']==payload and comment['author_membership_id']==owner_membership and not comment['edited']
inlines=comment['blocks'][0]['inlines']
assert any(i['text'].startswith('<img src=x onerror=alert(1)>') for i in inlines),'Markup stays literal text'
assert any(i['bold'] and i['text']=='bold' for i in inlines) and [i['href'] for i in inlines if i['href']]==['https://example.com/a']
assert call(comments)['items']==[comment]
assert call(path+'/cards/'+card['id'])['comment_count']==1
# An unsafe link that reached storage some other way still renders as plain text.
smuggled=call(comments,'POST',{'body':'safe text'},expected=201,key=key())
sql("UPDATE projects.comments SET body='[x](javascript:alert(1))' WHERE id='"+smuggled['id']+"'")
stored=next(c for c in call(comments)['items'] if c['id']==smuggled['id'])
assert 'javascript' not in json.dumps(stored['blocks']) or all(not i.get('href') for b in stored['blocks'] for i in b['inlines']),stored
call(path+'/comments/'+smuggled['id']+'/remove','POST',{'expected_version':smuggled['version']},key=key())
# Only the author edits; moderation is a separate grant.
own_comment=own.call(path+'/cards/'+hers['id']+'/comments','POST',{'body':'From the assignee'},expected=201)
call(path+'/comments/'+own_comment['id'],'PATCH',{'expected_version':own_comment['version'],'body':'Rewritten'},expected=403,key=key())
own.call(path+'/comments/'+own_comment['id'],'PATCH',{'expected_version':own_comment['version'],'body':'[bad](data:text/html,x)'},expected=400)
revised=own.call(path+'/comments/'+own_comment['id'],'PATCH',{'expected_version':own_comment['version'],'body':'Corrected'})
assert revised['edited'] and revised['body']=='Corrected'
own.call(path+'/comments/'+own_comment['id'],'PATCH',{'expected_version':own_comment['version'],'body':'Stale'},expected=409)
theirs=call(path+'/cards/'+hers['id']+'/comments','POST',{'body':'From the owner, private word'},expected=201,key=key())
own.call(path+'/comments/'+theirs['id']+'/remove','POST',{'expected_version':theirs['version']},expected=403)
retry_comment=key();once=call(comments,'POST',{'body':'Erase this secret'},expected=201,key=retry_comment)
wiped=call(path+'/comments/'+once['id']+'/remove','POST',{'expected_version':once['version']},key=key())
assert wiped['removed'] and wiped['body']=='' and wiped['blocks']==[]
assert sql("SELECT body FROM projects.comments WHERE id='"+once['id']+"'")==''
assert sql("SELECT count(*) FROM suite.idempotency_records WHERE workspace_id='"+f.workspace+"' AND (request::text LIKE '%Erase this secret%' OR result::text LIKE '%Erase this secret%')")=='0','Retry payloads do not keep erased text'
call(comments,'POST',{'body':'Erase this secret'},expected=409,key=retry_comment)
assert sql("SELECT count(*) FROM suite.audit_events WHERE details::text LIKE '%Erase this secret%'")=='0'
# A hidden task's comments stay hidden, including through cursors.
own.call(path+'/cards/'+card['id']+'/comments',expected=404)
own.call(path+'/cards/'+hers['id']+'/comments?after='+comment['id'],expected=404)
page=call(comments+'?limit=1');assert len(page['items'])==1 and page['next_cursor']
assert call(comments+'?limit=1&after='+page['next_cursor'])['items'][0]['id']!=page['items'][0]['id']
print('PASS: comments keep markup literal, refuse unsafe links, author/moderator rules, erased text and retries')

# Archiving freezes a project without touching its tasks.
sealed=call(projects,'POST',{'name':name+' sealed'},expected=201,key=key());sealed_path=projects+'/'+sealed['id']
sealed_card=add(sealed['id'],'Sealed task')
sealed=call(sealed_path+'/archive','POST',{'expected_version':sealed['version'],'archived':True},key=key())
assert sealed['archived'] and sealed['id'] not in [p['id'] for p in call(projects)['items']] and sealed['id'] in [p['id'] for p in call(projects+'?archived=true')['items']]
call(sealed_path+'/cards','POST',{'task':{'title':'x'}},expected=409,key=key())
call(sealed_path+'/cards/'+sealed_card['id']+'/comments','POST',{'body':'x'},expected=409,key=key())
assert sealed_card['id'] not in [i['card_id'] for i in all_work()]
assert call(base+'/tasks/'+sealed_card['task']['id'])['archived'] is False
sealed=call(sealed_path+'/archive','POST',{'expected_version':sealed['version'],'archived':False},key=key())
assert not sealed['archived']
# Nothing crosses workspaces.
foreign=sql("INSERT INTO suite.workspaces(name) VALUES('Projects foreign fixture') RETURNING id").splitlines()[0]
foreign_member=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Projects foreign member') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',id FROM u RETURNING id").splitlines()[0]
foreign_task=sql("INSERT INTO tasks.tasks(workspace_id,title,assigned_membership_id) VALUES('"+foreign+"','Foreign task','"+foreign_member+"') RETURNING id").splitlines()[0]
foreign_project=sql("INSERT INTO projects.projects(workspace_id,name,created_by_membership_id) VALUES('"+foreign+"','Foreign project','"+foreign_member+"') RETURNING id").splitlines()[0]
call('/api/v1/workspaces/'+foreign+'/projects',expected=403)
call(projects+'/'+foreign_project,expected=404)
call(projects+'/'+foreign_project+'/board',expected=404)
call(path+'/cards','POST',{'task_id':foreign_task},expected=404,key=key())
call(path+'/members','POST',{'membership_id':foreign_member},expected=400,key=key())
call(path+'/cards/'+card['id']+'/move','POST',{'expected_version':1,'column_id':secrets.token_hex(16),'before_card_id':None},expected=409,key=key())
# Another project's column or card is not a valid target.
other_column=call(projects+'/'+other_project['id']+'/columns')[0]['id']
now=call(path+'/cards/'+card['id'])
call(path+'/cards/'+card['id']+'/move','POST',{'expected_version':now['version'],'column_id':other_column,'before_card_id':None},expected=404,key=key())
call(projects+'/'+other_project['id']+'/cards/'+card['id'],expected=404)
try:
 sql("INSERT INTO projects.cards(workspace_id,project_id,task_id,column_id,position) VALUES('"+f.workspace+"','"+project['id']+"','"+foreign_task+"','"+todo+"',99)")
 raise AssertionError('Foreign task link accepted')
except subprocess.CalledProcessError:pass
print('PASS: archive freezes the board, tasks unaffected; cross-workspace and cross-project references refused')

# Export: documented collections, ordinary authority, stops with the application.
catalog={c['collection']:c for c in call(base+'/exports/collections')}
assert catalog['project_cards']['dependent_read_permissions']==['tasks.read'] and catalog['project_cards']['parent_required'] and catalog['projects']['supports_archive_selection']
def export(collection,expected=200,**query):return call(base+'/exports/page','POST',{'collection':collection,**query},expected=expected)
exported=[];request={'collection':'projects','limit':25}
while request:
 page=call(base+'/exports/page','POST',request);exported+=page['items'];request=page['next']
assert project['id'] in [p['id'] for p in exported]
assert export('projects',limit=1)['next']['after']
assert {m['id'] for m in export('project_members',parent_id=project['id'])['items']}=={m['id'] for m in call(path+'/members')}
assert len(export('project_columns',parent_id=project['id'])['items'])==3
assert card['id'] in [c['id'] for c in export('project_cards',parent_id=project['id'],limit=25)['items']]
assert removed_card['id'] not in [c['id'] for c in export('project_cards',parent_id=project['id'],ended=True)['items']]
page=export('project_cards',parent_id=project['id'],limit=1);assert call(base+'/exports/page','POST',page['next'])['items'][0]['id']!=page['items'][0]['id']
assert comment['id'] in [c['id'] for c in export('project_comments',parent_id=project['id'],limit=25)['items']]
export('project_cards',expected=400)
export('project_cards',parent_id=foreign_project,expected=404)
assert f.cli(['export','page'],{'collection':'project_columns','parent_id':project['id'],'limit':1})==export('project_columns',parent_id=project['id'],limit=1)
exporter=Person('Projects exporter',['workspace.data.export','projects.read:assigned','tasks.read:assigned'])
exporter.call(base+'/exports/page','POST',{'collection':'project_cards','parent_id':project['id']},expected=404)
assert project['id'] not in [p['id'] for p in exporter.call(base+'/exports/page','POST',{'collection':'projects','limit':25})['items']]
exporter.grants(['workspace.data.export','projects.read'])
exporter.call(base+'/exports/page','POST',{'collection':'project_cards','parent_id':project['id']},expected=403)
exporter.grants(['workspace.data.export','tasks.read'])
exporter.call(base+'/exports/page','POST',{'collection':'projects'},expected=403)
print('PASS: export pages, authority, scope and parents')

# CLI and MCP reach the same operations and refusals.
via_cli=f.cli(['projects','get-project','--project-id',project['id']])
assert via_cli['id']==project['id'] and via_cli['name']==name
retry=key();cli_project=f.cli(['projects','create-project','--idempotency-key',retry],{'name':name+' cli'})
assert f.cli(['projects','create-project','--idempotency-key',retry],{'name':name+' cli'})==cli_project
cli_card=f.cli(['projects','add-project-card','--project-id',cli_project['id'],'--idempotency-key',key()],{'task':{'title':'CLI task'}})
cli_columns=f.cli(['projects','list-project-columns','--project-id',cli_project['id']])
assert f.cli(['projects','move-project-card','--project-id',cli_project['id'],'--card-id',cli_card['id'],'--idempotency-key',key()],{'expected_version':cli_card['version'],'column_id':cli_columns[1]['id'],'before_card_id':None})['task']['status']=='in_progress'
assert f.cli(['projects','list-projects','--search',name+' cli'])['items'][0]['id']==cli_project['id']
assert f.cli(['projects','get-project-delivery','--project-id',cli_project['id'],'--status','in_progress'])['summary']['in_progress']==1
assert f.cli(['projects','add-project-comment','--project-id',cli_project['id'],'--card-id',cli_card['id'],'--idempotency-key',key()],{'body':'From the CLI'})['body']=='From the CLI'
def tool(name,arguments):
 result=f.mcp(name,arguments)
 assert not result.get('isError'),result
 return json.loads(result['content'][0]['text'])
assert tool('get_project_board',{'workspace_id':f.workspace,'project_id':cli_project['id']})['columns'][1]['cards'][0]['id']==cli_card['id']
retry=key();arguments={'workspace_id':f.workspace,'project_id':cli_project['id'],'card_id':cli_card['id'],'idempotency_key':retry,'change':{'body':'From MCP'}}
said=tool('add_project_comment',arguments)
assert said['body']=='From MCP' and tool('add_project_comment',arguments)==said
assert f.mcp('get_project',{'workspace_id':f.workspace,'project_id':foreign_project}).get('isError')
print('PASS: CLI and MCP parity, retries and refusals')

# Disabling Projects leaves shared Tasks usable and every project record in place.
counts=lambda:sql("SELECT (SELECT count(*) FROM projects.projects WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM projects.cards WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM projects.comments WHERE workspace_id='"+f.workspace+"'),(SELECT count(*) FROM projects.members WHERE workspace_id='"+f.workspace+"')")
snapshot=counts();whole=board(project['id'])
module('projects',False)
for target,method,payload_ in [(projects,'GET',None),(path,'GET',None),(path+'/board','GET',None),(path+'/delivery','GET',None),(projects+'/my-work','GET',None),(comments,'GET',None),(projects+'/recipes','GET',None),(projects,'POST',{'name':'x'}),(comments,'POST',{'body':'x'}),(path+'/cards/'+card['id']+'/move','POST',{'expected_version':1,'column_id':todo,'before_card_id':None})]:
 call(target,method,payload_,expected=403,key=key() if method=='POST' else None)
export('projects',expected=403)
assert f.mcp('list_projects',{'workspace_id':f.workspace}).get('isError')
task=call(base+'/tasks/'+card['task']['id'])
edited_task=call(base+'/tasks/'+task['id'],'PATCH',{'title':'Edited with Projects off','notes':task['notes'],'due_on':task['due_on'],'status':task['status'],'expected_version':task['version']},key=key())
assert edited_task['title']=='Edited with Projects off'
assert call(base+'/tasks','POST',{'title':'New with Projects off'},expected=201,key=key())['status']=='open'
assert counts()==snapshot,'Project data is retained'
module('projects',True)
restored_board=board(project['id'])
assert [[c['id'] for c in column['cards']] for column in restored_board['columns']]==[[c['id'] for c in column['cards']] for column in whole['columns']]
assert call(path+'/cards/'+card['id'])['task']['title']=='Edited with Projects off','Re-enabled boards read current task state'
print('PASS: disabling Projects keeps Tasks usable and data retained; re-enabling restores the board')

# Independence from CRM: a task linked to a deal and on a board reveals neither to the other side.
module('crm',True)
create=lambda p,b:call(base+p,'POST',b,expected=201,key=key())
sales='Sales '+secrets.token_hex(5)
category=create('/crm/categories',{'name':sales});planning=create('/crm/statuses',{'name':sales+' planning','parent_id':category['id']});won=create('/crm/statuses',{'name':sales+' won','parent_id':category['id']})
def new_deal(title,status=None,**more):return create('/crm/deals',{'title':title,'category_id':category['id'],'status_id':status or planning['id'],**more})
def transition(deal,status,expected=200,k=None):
 current=call(base+'/crm/deals/'+deal['id'])
 return call(base+'/crm/deals/'+deal['id'],'PATCH',{'title':current['title'],'category_id':category['id'],'status_id':status,'expected_version':current['version']},key=k or key(),expected=expected)
secret_deal=new_deal('Confidential deal '+secrets.token_hex(3))
linked_card=add(project['id'],'Task on a board and a deal')
linked_task=linked_card['task']['id']
link=call(base+'/crm/deals/'+secret_deal['id']+'/tasks','POST',{'expected_version':secret_deal['version'],'task_id':linked_task},expected=201,key=key())
# Projects and Tasks without CRM: the task, and nothing of the deal.
delivery_only=Person('Delivery only',['projects.read','tasks.read'])
seen=delivery_only.call(path+'/cards/'+linked_card['id'])
assert seen['task']['id']==linked_task
text=json.dumps([seen,delivery_only.call(path+'/board'),delivery_only.call(path+'/delivery'),delivery_only.call(base+'/tasks/'+linked_task)])
assert secret_deal['id'] not in text and secret_deal['title'] not in text and link['id'] not in text
delivery_only.call(base+'/crm/deals/'+secret_deal['id'],expected=403)
delivery_only.call(base+'/crm/deals/'+secret_deal['id']+'/tasks',expected=403)
# CRM and Tasks without Projects: the task, and nothing of the project.
sales_only=Person('Sales only',['crm.deals.read','tasks.read'])
links=sales_only.call(base+'/crm/deals/'+secret_deal['id']+'/tasks')['items']
assert [l['task']['id'] for l in links]==[linked_task]
text=json.dumps([links,sales_only.call(base+'/crm/deals/'+secret_deal['id']),sales_only.call(base+'/tasks/'+linked_task)])
assert project['id'] not in text and name not in text and linked_card['id'] not in text
sales_only.call(path,expected=403);sales_only.call(path+'/board',expected=403)
# Both applications without Tasks: neither shows the task.
no_tasks=Person('No tasks',['crm.deals.read','projects.read'])
assert no_tasks.call(base+'/crm/deals/'+secret_deal['id']+'/tasks')['items']==[]
no_tasks.call(path+'/cards/'+linked_card['id'],expected=404)
assert linked_task not in json.dumps(no_tasks.call(path+'/board'))
# CRM off changes nothing for delivery, and the project card survives ending the link.
module('crm',False)
assert delivery_only.call(path+'/cards/'+linked_card['id'])['id']==linked_card['id']
sales_only.call(base+'/crm/deals/'+secret_deal['id']+'/tasks',expected=403)
module('crm',True)
print('PASS: project and CRM visibility are independent over one shared task')

# The recipe: an administrator configures a deal status as the milestone.
recipes=projects+'/recipes'
call(recipes,'POST',{'name':name,'configuration':{'status_id':secrets.token_hex(16)}},expected=404,key=key())
call(recipes,'POST',{'name':name,'configuration':{'status_id':won['id'],'name_prefix':'x'*81}},expected=400,key=key())
call(recipes,'POST',{'name':name,'configuration':{'status_id':won['id'],'member_ids':[secrets.token_hex(16)]}},expected=400,key=key())
call(recipes,'POST',{'name':name,'configuration':{'status_id':won['id'],'member_ids':[foreign_member]}},expected=400,key=key())
activator=Person('Recipe activator',['projects.recipes.manage','projects.read','projects.create','projects.members.manage','crm.deals.read','crm.configuration.read'])
team=Person('Delivery team',['projects.read:assigned','tasks.read'])
configuration={'status_id':won['id'],'name_prefix':'Delivery','include_deal_owner':True,'member_ids':[team.membership]}
recipe=activator.call(recipes,'POST',{'name':name,'configuration':configuration},expected=201)
assert recipe['active'] is False and recipe['revision']['number']==1
recipe_path=recipes+'/'+recipe['id']
# Nothing fires before activation, and activating needs every permission a run uses.
quiet=new_deal('Before activation',won['id'])
assert call(recipe_path+'/runs')['items']==[]
lacking=Person('Lacking',['projects.recipes.manage','projects.read','crm.configuration.read'])
lacking.call(recipe_path+'/active','POST',{'expected_version':recipe['version'],'active':True},expected=403)
assert activator.call(recipe_path)['active'] is False
recipe=activator.call(recipe_path+'/active','POST',{'expected_version':recipe['version'],'active':True})
assert recipe['active'] and recipe['activated_by_membership_id']==activator.membership
duplicate=call(recipes,'POST',{'name':name+' twin','configuration':{'status_id':won['id']}},expected=201,key=key())
call(recipes+'/'+duplicate['id']+'/active','POST',{'expected_version':duplicate['version'],'active':True},expected=409,key=key())
def run_of(deal):
 rows=call(recipe_path+'/runs?limit=100')['items']
 return next((r for r in rows if r['deal_id']==deal['id']),None)
def settled(deal,seconds=30):return wait_for(lambda:(lambda r:r if r and (r['state']=='completed' or r['job_state'] in ('blocked','exhausted','cancelled')) else None)(run_of(deal)),'conversion',seconds)
def conversions(deal):return sql("SELECT count(*) FROM projects.deal_conversions WHERE deal_id='"+deal['id']+"'")
def projects_of(title):return sql("SELECT count(*) FROM projects.projects WHERE workspace_id='"+f.workspace+"' AND name='"+title+"'")
deal=new_deal('Fit-out '+secrets.token_hex(3))
assert run_of(deal) is None,'Other statuses convert nothing'
current=call(base+'/crm/deals/'+deal['id']);change={'title':current['title'],'category_id':category['id'],'status_id':won['id'],'expected_version':current['version']}
retry=key();assert call(base+'/crm/deals/'+deal['id'],'PATCH',change,key=retry)==call(base+'/crm/deals/'+deal['id'],'PATCH',change,key=retry)
run=settled(deal)
assert run['state']=='completed' and run['project_id'] and run['revision_number']==1,run
made=call(projects+'/'+run['project_id'])
assert made['name']=='Delivery: '+deal['title'] and made['source_deal_id']==deal['id'] and made['status']=='active'
assert {m['id'] for m in call(projects+'/'+made['id']+'/members')}=={team.membership,owner_membership},'Configured members and the deal owner join'
assert [c['name'] for c in call(projects+'/'+made['id']+'/columns')]==['To do','In progress','Done']
assert team.call(projects+'/'+made['id'])['source_deal_id'] is None,'A member without CRM access never sees the deal link'
assert sql("SELECT source_interface FROM suite.audit_events WHERE record_id='"+made['id']+"' AND action='projects.created'")=='worker'
job=call(base+'/jobs/'+run['job_id'])
assert job['kind']=='projects.deal_conversion' and job['state']=='completed' and job['projects_context']['run_id']==run['id']
# Retrying, re-entering the status and racing cannot make a second project.
call(recipe_path+'/runs/'+run['id']+'/retry','POST',{'expected_version':run['version']},expected=409,key=key())
call(base+'/jobs/'+run['job_id'],'PATCH',{'expected_version':job['version'],'action':'resume'},expected=409,key=key())
transition(deal,planning['id']);transition(deal,won['id']);time.sleep(1)
assert conversions(deal)=='1' and projects_of(made['name'])=='1'
# A retried business change via the worker's own re-execution also does nothing new.
sql("UPDATE suite.jobs SET state='blocked' WHERE id='"+run['job_id']+"'")
stale=call(recipe_path+'/runs?limit=100')['items'];rerun=next(r for r in stale if r['id']==run['id'])
call(recipe_path+'/runs/'+run['id']+'/retry','POST',{'expected_version':rerun['version']},key=key())
wait_for(lambda:call(base+'/jobs/'+run['job_id'])['state']=='completed','re-run')
assert conversions(deal)=='1' and projects_of(made['name'])=='1',"A completed conversion is not redone"
racing=[new_deal('Race '+secrets.token_hex(3)) for _ in range(4)]
with ThreadPoolExecutor(max_workers=8) as pool:
 def race_transition(d):
  current=call(base+'/crm/deals/'+d['id'])
  body_={'title':current['title'],'category_id':category['id'],'status_id':won['id'],'expected_version':current['version']}
  list(ThreadPoolExecutor(max_workers=3).map(lambda _:call(base+'/crm/deals/'+d['id'],'PATCH',body_,key=shared_keys[d['id']]),range(3)))
 shared_keys={d['id']:key() for d in racing}
 list(pool.map(race_transition,racing))
for d in racing:
 assert settled(d)['state']=='completed' and conversions(d)=='1' and projects_of('Delivery: '+d['title'])=='1'
print('PASS: one project per configured conversion through retries, re-entry, job resume and concurrency')

# A failing step rolls back and retries without leaving a project behind.
sql("""CREATE OR REPLACE FUNCTION projects.probe_fail() RETURNS trigger LANGUAGE plpgsql AS $$
 BEGIN IF NEW.name LIKE 'Delivery: Fault%' THEN RAISE EXCEPTION 'injected fault'; END IF; RETURN NEW; END $$;
 CREATE TRIGGER probe_fail BEFORE INSERT ON projects.projects FOR EACH ROW EXECUTE FUNCTION projects.probe_fail();""")
try:
 faulty=new_deal('Fault '+secrets.token_hex(3));transition(faulty,won['id'])
 failed=settled(faulty,seconds=90)
 assert failed['job_state']=='exhausted' and failed['error_code']=='database' and failed['project_id'] is None,failed
finally:sql('DROP TRIGGER probe_fail ON projects.projects;DROP FUNCTION projects.probe_fail();')
assert projects_of('Delivery: '+faulty['title'])=='0'
call(recipe_path+'/runs/'+failed['id']+'/retry','POST',{'expected_version':failed['version']},key=key())
assert settled(faulty)['state']=='completed' and projects_of('Delivery: '+faulty['title'])=='1'
# A worker outage after the status commit keeps the work; a crashed lease is reclaimed.
subprocess.run(['docker','compose','stop','worker'],cwd=ROOT,env=f.environment,check=True,capture_output=True)
try:
 outage=new_deal('Outage '+secrets.token_hex(3));transition(outage,won['id']);time.sleep(1)
 pending=run_of(outage);assert pending['state']=='queued' and pending['project_id'] is None
 sql("UPDATE suite.jobs SET state='running',attempts=1,lease_token=gen_random_uuid(),lease_until=now()-interval '1 second' WHERE id='"+pending['job_id']+"'")
finally:subprocess.run(['docker','compose','start','worker'],cwd=ROOT,env=f.environment,check=True,capture_output=True)
assert settled(outage,seconds=90)['state']=='completed' and projects_of('Delivery: '+outage['title'])=='1'
# Disabling either participant during pending work blocks it visibly and it cannot run.
subprocess.run(['docker','compose','stop','worker'],cwd=ROOT,env=f.environment,check=True,capture_output=True)
try:
 paused=new_deal('Paused '+secrets.token_hex(3));transition(paused,won['id'])
 pending=run_of(paused);assert pending['job_state']=='queued'
 module('projects',False)
 job=call(base+'/jobs/'+pending['job_id']);assert job['state']=='blocked' and job['error_code']=='module_disabled',job
 call(base+'/jobs/'+pending['job_id'],'PATCH',{'expected_version':job['version'],'action':'resume'},expected=403,key=key())
 module('projects',True)
 paused_crm=new_deal('Paused CRM '+secrets.token_hex(3));transition(paused_crm,won['id'])
 module('crm',False)
 assert sql("SELECT j.state,j.error_code FROM projects.deal_conversions c JOIN suite.jobs j ON j.id=c.job_id WHERE c.deal_id='"+paused_crm['id']+"'")=='blocked|module_disabled'
 module('crm',True)
finally:subprocess.run(['docker','compose','start','worker'],cwd=ROOT,env=f.environment,check=True,capture_output=True)
time.sleep(1);held=run_of(paused);assert held['job_state']=='blocked' and projects_of('Delivery: '+paused['title'])=='0','Re-enabling does not replay old work implicitly'
call(recipe_path+'/runs/'+held['id']+'/retry','POST',{'expected_version':held['version']},key=key())
assert settled(paused)['state']=='completed' and projects_of('Delivery: '+paused['title'])=='1'
print('PASS: rollback/retry, worker outage durability, crashed lease, pending work blocked by disablement')

# A milestone reached while Projects is off is recorded, blocked visibly, and retriable.
module('projects',False)
offline=new_deal('Offline '+secrets.token_hex(3));moved_deal=transition(offline,won['id'])
assert moved_deal['status_id']==won['id'],'CRM keeps working with Projects off'
blocked_job=wait_for(lambda:(lambda j:j if j and j[1]=='blocked' else None)((lambda r:(r.split('|') if r else None))(sql("SELECT j.id,j.state,j.error_code FROM projects.deal_conversions c JOIN suite.jobs j ON j.id=c.job_id WHERE c.deal_id='"+offline['id']+"'"))),'blocked job')
assert blocked_job[2]=='module_disabled' and projects_of('Delivery: '+offline['title'])=='0'
module('projects',True)
blocked=settled(offline);assert blocked['job_state']=='blocked' and blocked['error_code']=='module_disabled' and blocked['project_id'] is None
# A retry refused while CRM is off explains itself.
module('crm',False)
call(recipe_path+'/runs/'+blocked['id']+'/retry','POST',{'expected_version':blocked['version']},expected=409,key=key())
call(recipe_path,expected=200)
module('crm',True)
call(recipe_path+'/runs/'+blocked['id']+'/retry','POST',{'expected_version':blocked['version']},key=key())
assert settled(offline)['state']=='completed' and projects_of('Delivery: '+offline['title'])=='1'
# Revoked authority blocks the run visibly; restoring it allows the same conversion.
full=['projects.recipes.manage','projects.read','projects.create','projects.members.manage','crm.deals.read','crm.configuration.read']
activator.grants([g for g in full if g!='projects.create'])
revoked=new_deal('Revoked '+secrets.token_hex(3));transition(revoked,won['id'])
failed=settled(revoked);assert failed['job_state']=='blocked' and failed['error_code']=='authority' and failed['project_id'] is None,failed
assert projects_of('Delivery: '+revoked['title'])=='0'
activator.grants([g for g in full if g!='crm.deals.read'])
call(recipe_path+'/runs/'+failed['id']+'/retry','POST',{'expected_version':failed['version']},key=key())
failed=wait_for(lambda:(lambda r:r if r['job_state']=='blocked' else None)(run_of(revoked)),'second block');assert failed['error_code']=='authority',failed
activator.grants(full)
call(recipe_path+'/runs/'+failed['id']+'/retry','POST',{'expected_version':failed['version']},key=key())
assert settled(revoked)['state']=='completed' and projects_of('Delivery: '+revoked['title'])=='1'
# Membership removal ends authority too, and invalid configuration is a visible failure.
gone_member=Person('Departing member',['tasks.read'])
broken=activator.call(recipe_path,'PATCH',{'name':name,'configuration':{**configuration,'member_ids':[team.membership,gone_member.membership]},'expected_version':call(recipe_path)['version']})
assert broken['revision']['number']==2
sql("UPDATE suite.memberships SET state='suspended' WHERE id='"+gone_member.membership+"'")
invalid=new_deal('Invalid '+secrets.token_hex(3));transition(invalid,won['id'])
failed=settled(invalid);assert failed['job_state']=='blocked' and failed['error_code']=='invalid_input' and failed['revision_number']==2,failed
sql("UPDATE suite.memberships SET state='active' WHERE id='"+gone_member.membership+"'")
call(recipe_path+'/runs/'+failed['id']+'/retry','POST',{'expected_version':failed['version']},key=key())
assert settled(invalid)['state']=='completed'
sql("UPDATE suite.memberships SET state='suspended' WHERE id='"+activator.membership+"'")
try:
 removed=new_deal('Removed authority '+secrets.token_hex(3));transition(removed,won['id'])
 failed=settled(removed);assert failed['job_state']=='blocked' and failed['error_code']=='authority',failed
finally:sql("UPDATE suite.memberships SET state='active' WHERE id='"+activator.membership+"'")
call(recipe_path+'/runs/'+failed['id']+'/retry','POST',{'expected_version':failed['version']},key=key())
assert settled(removed)['state']=='completed'
# Run references follow the viewer's own authority.
limited=Person('Recipe viewer',['projects.recipes.manage'])
listed=limited.call(recipe_path+'/runs?limit=100')['items']
assert listed and all(r['deal_id'] is None and r['project_id'] is None for r in listed)
# Deactivation stops new conversions without touching earlier ones, and no scoped recipe access exists.
latest=call(recipe_path)
activator.call(recipe_path+'/active','POST',{'expected_version':latest['version'],'active':False})
silent=new_deal('Inactive '+secrets.token_hex(3));transition(silent,won['id']);time.sleep(.5)
assert conversions(silent)=='0' and projects_of('Delivery: '+deal['title'])=='1'
reader.call(recipes,expected=403)
# With CRM off, Projects and its finished conversions remain usable; the deal link is withheld.
module('crm',False)
assert call(projects+'/'+made['id'])['source_deal_id'] is None
assert call(recipe_path+'/runs?limit=100')['items'][0]['deal_id'] is None
assert call(projects,'POST',{'name':name+' crm off'},expected=201,key=key())
call(recipes,'POST',{'name':name+' crm off','configuration':configuration},expected=403,key=key())
module('crm',True)
assert call(projects+'/'+made['id'])['source_deal_id']==deal['id']
print('PASS: recipe blocked visibly by disabled modules, revoked authority and invalid configuration; each retry stays one conversion')
print('PASS: Projects journey')

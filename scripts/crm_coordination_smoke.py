#!/usr/bin/env python3
"""Pinned CRM coordination creates ordinary Tasks atomically."""
import secrets
from integration_fixture import Fixture
f=Fixture();call=f.call;base='/api/v1/workspaces/'+f.workspace
name='Coordination '+secrets.token_hex(5)
def key():return secrets.token_hex(16)
def create(path,body):return call(base+path,'POST',body,expected=201,key=key())
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if not module['enabled']:call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key())
work=create('/tasks/templates',{'name':name,'title':name+' check requirements','assignment':'context_owner','due_offset_days':2})
request={'name':name,'items':[{'label':'Requirements','task_template_revision_id':work['revision']['id']}]}
template=create('/crm/coordination-templates',request)
category=create('/crm/categories',{'name':name});status=create('/crm/statuses',{'name':name,'parent_id':category['id']})
deal=create('/crm/deals',{'title':name,'category_id':category['id'],'status_id':status['id']})
path=base+'/crm/deals/'+deal['id']+'/coordination';retry=key();apply={'expected_version':deal['version'],'revision_id':template['revision']['id'],'base_on':'2028-02-28'}
instance=call(path,'POST',apply,expected=201,key=retry)
assert call(path,'POST',apply,expected=201,key=retry)==instance
assert instance['items'][0]['task']['due_on']=='2028-03-01'
assert instance['items'][0]['task']['assigned_membership_id']==deal['owner_membership_id']
assert call(base+'/tasks/'+instance['items'][0]['task']['id'])==instance['items'][0]['task']
assert call(path+'/'+instance['id'])==instance
print('PASS: pinned coordination creates ordinary Tasks with contextual assignment and stable retries')
# Definition changes preserve old task specifications and instance identities.
template_path=base+'/crm/coordination-templates/'+template['id'];deal_path=base+'/crm/deals/'+deal['id']
second=create('/tasks/templates',{'name':name+' second','title':name+' verify','assignment':'initiator','due_offset_days':-1})
request2={**request,'items':[*request['items'],{'label':'Verification','task_template_revision_id':second['revision']['id']}]}
changed=call(template_path,'PATCH',{**request2,'expected_version':1},key=key())
assert changed['revision']['number']==2 and call(path+'/'+instance['id'])==instance
assert call(template_path+'/revisions?limit=1')['next_cursor']==template['revision']['id']
assert call(template_path+'/revisions?limit=1&after='+template['revision']['id'])['items']==[changed['revision']]
renamed=call(template_path,'PATCH',{**request2,'name':name+' renamed','expected_version':changed['version']},key=key())
assert renamed['revision']==changed['revision']
def application(revision=None):return {'expected_version':call(deal_path)['version'],'revision_id':revision or changed['revision']['id'],'base_on':'2028-03-01'}
new_input=application();new_key=key();pair=call(path,'POST',new_input,expected=201,key=new_key)
assert [i['label'] for i in pair['items']]==['Requirements','Verification']
assert [i['task']['due_on'] for i in pair['items']]==['2028-03-03','2028-02-29']
assert call(deal_path)['version']==new_input['expected_version']+2
assert pair['template_name']==name
summary=call(path+'?limit=1');assert summary['items'][0]['id']==instance['id'] and summary['next_cursor']==instance['id']
assert 'items' not in summary['items'][0]
assert call(path+'?limit=1&after='+instance['id'])['items'][0]['id']==pair['id']
call(path+'?after='+template['id'],expected=404);call(path+'?limit=26',expected=400)
# Task lifecycle remains owned by Tasks; ending a link retains checklist evidence.
task=pair['items'][0]['task'];task_path=base+'/tasks/'+task['id']
fields={k:task[k] for k in ['title','notes','due_on','assigned_membership_id']}
done=call(task_path,'PATCH',{**fields,'status':'completed','expected_version':task['version']},key=key())
assert call(path+'/'+pair['id'])['items'][0]['task']==done
link=pair['items'][0]['link_id']
call(deal_path+'/tasks/'+link,'PATCH',{'expected_version':call(deal_path)['version'],'ended':True},key=key())
assert call(path+'/'+pair['id'])['items'][0]['link_ended'] and call(task_path)==done
retired=call(template_path,'PATCH',{**request2,'expected_version':renamed['version'],'archived':True},key=key())
call(path,'POST',application(),expected=409,key=key())
assert call(path,'POST',new_input,expected=201,key=new_key)==pair
assert call(path+'/'+pair['id'])['items'][0]['task']==done
call(template_path,'PATCH',{**request2,'expected_version':retired['version'],'archived':False},key=key())
call(deal_path+'/archive','POST',{'expected_version':call(deal_path)['version'],'archived':True},key=key())
call(path,'POST',application(),expected=409,key=key());assert call(path+'/'+pair['id'])
call(deal_path+'/archive','POST',{'expected_version':call(deal_path)['version'],'archived':False},key=key())
print('PASS: ordered immutable checklist revisions, calendar offsets, summary pagination, task lifecycle and archive/link retention')
# Each action retains independent authority, including the original retry result.
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Checklist assignee') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
def grant(action,scope):f.sql("UPDATE suite.role_permissions SET record_scope='"+scope+"' WHERE role_id='"+role+"' AND permission_key='"+action+"'")
current=call(task_path);call(task_path,'PATCH',{**fields,'status':current['status'],'assigned_membership_id':other,'expected_version':current['version']},key=key())
grant('tasks.read','assigned')
try:
 visible=call(path+'/'+pair['id']);assert [i['label'] for i in visible['items']]==['Verification']
 assert task['id'] not in str(visible) and 'total' not in visible
 call(path,'POST',new_input,expected=404,key=new_key)
finally:grant('tasks.read','workspace')
grant('tasks.create','assigned')
try:call(path,'POST',new_input,expected=404,key=new_key)
finally:grant('tasks.create','workspace')
for permission in ['crm.deals.coordination.apply','crm.deals.tasks.manage','crm.configuration.read','tasks.configuration.read','tasks.create','tasks.read']:
 f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='"+permission+"'")
 try:
  call(path,'POST',application(),expected=403,key=key())
  call(path,'POST',new_input,expected=403,key=new_key)
  if permission=='tasks.read':assert call(path+'/'+pair['id'])['items']==[]
 finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','"+permission+"')")
hidden=create('/crm/deals',{'title':name+' other owner','category_id':category['id'],'status_id':status['id'],'owner_membership_id':other})
for permission in ['crm.deals.read','crm.deals.coordination.apply','crm.deals.tasks.manage']:
 grant(permission,'assigned')
 try:call(base+'/crm/deals/'+hidden['id']+'/coordination','POST',{**application(),'expected_version':1},expected=404,key=key())
 finally:grant(permission,'workspace')
module=next(m for m in call(base+'/modules') if m['key']=='crm');module=call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key())
try:
 call(path,expected=403);call(path,'POST',new_input,expected=403,key=new_key);assert call(task_path)
finally:call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key())
print('PASS: independent grants, assigned scope, hidden task projection, retry reauthorization and disabled module behavior')
# Failure after task/link insertion must roll back every intermediate record.
def counts():return f.sql("SELECT (SELECT count(*) FROM tasks.tasks),(SELECT count(*) FROM crm.deal_tasks),(SELECT count(*) FROM crm.deal_coordination_instances),(SELECT count(*) FROM crm.deal_coordination_items),(SELECT count(*) FROM suite.audit_events),(SELECT count(*) FROM suite.idempotency_records)")
before=counts();pending=application();failed_key=key()
f.sql('REVOKE INSERT ON crm.deal_coordination_items FROM looski_runtime')
try:call(path,'POST',pending,expected=503,key=failed_key)
finally:f.sql('GRANT INSERT ON crm.deal_coordination_items TO looski_runtime')
assert counts()==before and call(deal_path)['version']==pending['expected_version']
recovered=call(path,'POST',pending,expected=201,key=failed_key)
assert call(path,'POST',pending,expected=201,key=failed_key)==recovered
# Later step fails after first step was successfully staged in the transaction.
second_path=base+'/tasks/templates/'+second['id']
call(second_path,'PATCH',{'name':second['name'],'title':second['revision']['title'],'assignment':'initiator','due_offset_days':-1,'expected_version':1,'archived':True},key=key())
before=counts();pending=application()
call(path,'POST',pending,expected=409,key=key());assert counts()==before
assert call(deal_path)['version']==pending['expected_version']
call(second_path,'PATCH',{'name':second['name'],'title':second['revision']['title'],'assignment':'initiator','due_offset_days':-1,'expected_version':2,'archived':False},key=key())
for bad in [{'items':[]},{'items':request['items']*26},{'items':[{'label':'','task_template_revision_id':work['revision']['id']}]},{'items':[{'label':'Missing','task_template_revision_id':str(__import__('uuid').uuid4())}]}]:
 call(base+'/crm/coordination-templates','POST',{**request,**bad},expected=404 if bad['items'] and bad['items'][0]['label']=='Missing' else 400,key=key())
call(path,'POST',{**application(),'base_on':'2028-02-30'},expected=400,key=key())
# Composite keys reject cross-instance/deal and cross-revision steps at the database boundary.
f.sql("DO $$ BEGIN BEGIN UPDATE crm.deal_coordination_items SET revision_id='"+changed['revision']['id']+"' WHERE instance_id='"+instance['id']+"'; RAISE EXCEPTION 'Mismatched revision accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
f.sql("DO $$ BEGIN BEGIN UPDATE crm.deal_coordination_items SET deal_id='"+hidden['id']+"' WHERE instance_id='"+instance['id']+"'; RAISE EXCEPTION 'Mismatched deal accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
call(base+'/crm/deals/'+hidden['id']+'/coordination/'+instance['id'],expected=404)
print('PASS: mid-composition and late-step rollback, validation and typed composite reference constraints')
# Concurrent retries produce one application; competing versions cannot leave orphan tasks.
import json
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request,urlopen
from urllib.error import HTTPError
for same_key in [True,False]:
 pending=application();shared_key=key();before=int(f.sql('SELECT count(*) FROM tasks.tasks'))
 def race(number):
  req=Request(f.base+path,method='POST',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':shared_key if same_key else key()},data=json.dumps(pending).encode())
  try:r=urlopen(req,timeout=20)
  except HTTPError as e:r=e
  with r:return r.status,json.loads(r.read())
 with ThreadPoolExecutor(2) as pool:results=list(pool.map(race,[1,2]))
 assert sorted(r[0] for r in results)==([201,201] if same_key else [201,409]),results
 if same_key:assert results[0][1]==results[1][1]
 assert int(f.sql('SELECT count(*) FROM tasks.tasks'))==before+2
# Maximum-sized work results can be stored/replayed; listing remains bounded summaries.
large_work=create('/tasks/templates',{'name':name+' large','title':name+' large','notes':'z'*10000,'assignment':'initiator','due_offset_days':0})
large=create('/crm/coordination-templates',{'name':name+' maximum','items':[{'label':'Step '+str(n),'task_template_revision_id':large_work['revision']['id']} for n in range(25)]})
large_key=key();pending=application(large['revision']['id']);large_result=call(path,'POST',pending,expected=201,key=large_key)
assert len(large_result['items'])==25 and len(json.dumps(large_result))>65536
assert call(path,'POST',pending,expected=201,key=large_key)==large_result
assert all('items' not in i for i in call(path)['items'])
assert f.cli(['crm','coordination-template',template['id']])['id']==template['id']
assert not f.mcp('crm_deal_coordination_instance',{'workspace_id':f.workspace,'id':deal['id'],'instance_id':pair['id']}).get('isError')
print('PASS: concurrent idempotency/version protection, bounded 25-task large replay and CLI/MCP parity')
# SSR creation, revision editing and application keep drafts on rejected operations.
from browser_forms import Forms
from urllib.parse import urlencode,urlparse
def browser(route,values=None,expected=200):
 req=Request(f.base+route,data=None if values is None else urlencode(values,doseq=True).encode(),headers={'Origin':f.base,'Accept':'text/html'})
 try:r=f.browser.open(req,timeout=20)
 except HTTPError as e:r=e
 with r:
  html=r.read().decode();assert r.status==expected,(route,r.status,html[:250]);return Forms(html),html,r.url
web='/workspaces/'+f.workspace+'/crm';listing=web+'/coordination-templates'
forms,_,_=browser(listing);draft=forms.form('name');draft.update(name=name+' browser',item_label='Browser work',task_template_revision_id='bad')
forms,_,_=browser(listing,draft,400);assert forms.form('name')==draft
draft['task_template_revision_id']=work['revision']['id'];forms,_,url=browser(listing,draft);browser_template=urlparse(url).path.rsplit('/',1)[1]
stale=forms.form('name');current=call(base+'/crm/coordination-templates/'+browser_template)
call(base+'/crm/coordination-templates/'+browser_template,'PATCH',{'name':current['name']+' server','items':[request['items'][0]],'expected_version':current['version']},key=key())
forms,html,_=browser(listing+'/'+browser_template,stale,409);assert forms.form('name')==stale and 'Discard draft' in html
# Multiple repeated form controls preserve ordering through the server action.
forms,_,_=browser(listing+'/'+browser_template);draft=forms.form('name');draft['item_label']=['First browser','Second browser'];draft['task_template_revision_id']=[work['revision']['id'],second['revision']['id']]
browser(listing+'/'+browser_template,draft)
assert [i['label'] for i in call(base+'/crm/coordination-templates/'+browser_template)['revision']['items']]==draft['item_label']
execution_path=web+'/deals/'+deal['id']+'/coordination';forms,_,_=browser(execution_path);execution=forms.form('base_on');execution.update(revision_id=changed['revision']['id'],base_on='2028-02-30')
forms,_,_=browser(execution_path,execution,400);assert forms.form('base_on')==execution
execution['base_on']='2028-03-01';forms,html,url=browser(execution_path,execution)
assert 'Requirements' in html and 'Verification' in html and 'Visible work' in html
assert urlparse(url).path.startswith(execution_path+'/')
print('PASS: SSR retained invalid/stale drafts, ordered repeated form controls, checklist application and task projection')

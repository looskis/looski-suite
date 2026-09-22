#!/usr/bin/env python3
"""Independent task visibility and atomic CRM follow-up composition."""
import secrets
from integration_fixture import Fixture
f=Fixture();call=f.call;base='/api/v1/workspaces/'+f.workspace
name='CRM task '+secrets.token_hex(5)
def key():return secrets.token_hex(16)
def create(path,body):return call(base+path,'POST',body,expected=201,key=key())
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if not module['enabled']:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
pipeline=create('/crm/pipelines',{'name':name});stage=create('/crm/stages',{'name':name,'parent_id':pipeline['id'],'outcome':'open'})
category=create('/crm/categories',{'name':name});status=create('/crm/statuses',{'name':name,'parent_id':category['id']})
for kind,body in [('opportunities',{'title':name,'pipeline_id':pipeline['id'],'stage_id':stage['id']}),('deals',{'title':name,'category_id':category['id'],'status_id':status['id']})]:
 record=create('/crm/'+kind,body);path=base+'/crm/'+kind+'/'+record['id']
 retry=key();change={'expected_version':record['version'],'task':{'title':name+' follow-up','due_on':'2028-03-01'}}
 followup=call(path+'/followups','POST',change,expected=201,key=retry)
 assert call(path+'/followups','POST',change,expected=201,key=retry)==followup
 assert call(base+'/tasks/'+followup['task']['id'])==followup['task']
 assert call(path)['version']==2 and call(path+'/tasks')['items']==[followup]
 print('PASS: '+kind+' atomic follow-up, task identity and original-result retry')
 task=followup['task'];link_id=followup['id'];link_path=path+'/tasks/'+link_id
 unchanged=call(base+'/tasks/'+task['id'])
 call(path+'/tasks','POST',{'expected_version':2,'task_id':task['id']},expected=409,key=key())
 ended=call(link_path,'PATCH',{'expected_version':2,'ended':True},key=key());assert ended['ended']
 assert call(base+'/tasks/'+task['id'])==unchanged
 assert call(path+'/tasks')['items']==[] and call(path+'/tasks?ended=true')['items'][0]['id']==link_id
 assert [h['ended'] for h in call(link_path+'/history')['items']]==[False,True]
 restored=call(link_path,'PATCH',{'expected_version':3,'ended':False},key=key());assert not restored['ended']
 assert call(path+'/tasks?limit=1')['items'][0]['id']==link_id
 call(link_path+'/history?after='+task['id'],expected=404)
 # Mid-composition failure must roll back the already-inserted task/history/audit.
 singular='opportunity' if kind=='opportunities' else 'deal'
 before_count=f.sql("SELECT count(*) FROM tasks.tasks WHERE workspace_id='"+f.workspace+"'")
 before_audit=f.sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"'")
 version=call(path)['version'];failed_key=key();pending={'expected_version':version,'task':{'title':name+' rollback'}}
 f.sql('REVOKE INSERT ON crm.'+singular+'_tasks FROM looski_runtime')
 try:call(path+'/followups','POST',pending,expected=503,key=failed_key)
 finally:f.sql('GRANT INSERT ON crm.'+singular+'_tasks TO looski_runtime')
 assert f.sql("SELECT count(*) FROM tasks.tasks WHERE workspace_id='"+f.workspace+"'")==before_count
 assert f.sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"'")==before_audit
 assert call(path)['version']==version
 recovered=call(path+'/followups','POST',pending,expected=201,key=failed_key)
 assert call(path+'/followups','POST',pending,expected=201,key=failed_key)==recovered
 # Independent Tasks scopes and parent scopes intersect without disclosing hidden links.
 role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
 other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('CRM task assignee') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
 def grant(action,scope):f.sql("UPDATE suite.role_permissions SET record_scope='"+scope+"' WHERE role_id='"+role+"' AND permission_key='"+action+"'")
 def edit_task(task_id,assignee):
  current=call(base+'/tasks/'+task_id)
  fields={field:current[field] for field in ['title','notes','due_on','status']}
  return call(base+'/tasks/'+task_id,'PATCH',{**fields,'assigned_membership_id':assignee,'expected_version':current['version']},key=key())
 edit_task(task['id'],other)
 grant('tasks.read','assigned')
 try:
  assert all(item['id']!=link_id for item in call(path+'/tasks')['items'])
  call(path+'/tasks?after='+link_id,expected=404)
  call(link_path+'/history',expected=404)
  call(link_path,'PATCH',{'expected_version':call(path)['version'],'ended':True},expected=404,key=key())
  call(path+'/followups','POST',change,expected=404,key=retry)
 finally:grant('tasks.read','workspace')
 grant('tasks.create','assigned')
 try:
  call(path+'/followups','POST',change,expected=404,key=retry)
  call(path+'/followups','POST',{'expected_version':call(path)['version'],'task':{'title':'Denied assignment','assigned_membership_id':other}},expected=403,key=key())
 finally:grant('tasks.create','workspace')
 edit_task(task['id'],task['assigned_membership_id'])
 hidden_parent=create('/crm/'+kind,{**body,'owner_membership_id':other});hidden_path=base+'/crm/'+kind+'/'+hidden_parent['id']
 grant('crm.'+kind+'.tasks.manage','assigned')
 try:call(hidden_path+'/tasks','POST',{'expected_version':1,'task_id':task['id']},expected=404,key=key())
 finally:grant('crm.'+kind+'.tasks.manage','workspace')
 # Missing Tasks permission returns a uniformly empty link list and hidden history.
 f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='tasks.read'")
 try:
  assert call(path+'/tasks')['items']==[];call(link_path+'/history',expected=404)
  call(link_path,'PATCH',{'expected_version':call(path)['version'],'ended':True},expected=403,key=key())
 finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','tasks.read')")
 # Hidden candidate batches cannot truncate a visible page or provide a hidden cursor.
 hidden_link=f.sql("WITH t AS (INSERT INTO tasks.tasks(workspace_id,title,assigned_membership_id) SELECT '"+f.workspace+"','Hidden linked task','"+other+"' FROM generate_series(1,105) RETURNING id), l AS (INSERT INTO crm."+singular+"_tasks(workspace_id,"+singular+"_id,task_id,created_by_membership_id) SELECT '"+f.workspace+"','"+record['id']+"',id,'"+other+"' FROM t RETURNING id) SELECT id FROM l LIMIT 1")
 late=create('/tasks',{'title':name+' later visible'})
 late_link=call(path+'/tasks','POST',{'expected_version':call(path)['version'],'task_id':late['id']},expected=201,key=key())
 grant('tasks.read','assigned')
 try:
  page=call(path+'/tasks?limit=2');assert len(page['items'])==2 and page['next_cursor']
  page2=call(path+'/tasks?limit=2&after='+page['next_cursor']);assert [l['id'] for l in page2['items']]==[late_link['id']] and page2['next_cursor'] is None
  call(path+'/tasks?after='+hidden_link,expected=404)
 finally:grant('tasks.read','workspace')
 # Archived references remain visible; new activation requires active parent and task.
 current=call(base+'/tasks/'+task['id']);call(base+'/tasks/'+task['id']+'/archive','POST',{'expected_version':current['version'],'archived':True},key=key())
 call(link_path,'PATCH',{'expected_version':call(path)['version'],'ended':True},key=key())
 call(link_path,'PATCH',{'expected_version':call(path)['version'],'ended':False},expected=409,key=key())
 assert call(link_path+'/history')['link']['task']['archived']
 current=call(base+'/tasks/'+task['id']);call(base+'/tasks/'+task['id']+'/archive','POST',{'expected_version':current['version'],'archived':False},key=key())
 call(link_path,'PATCH',{'expected_version':call(path)['version'],'ended':False},key=key())
 call(path+'/archive','POST',{'expected_version':call(path)['version'],'archived':True},key=key())
 call(link_path,'PATCH',{'expected_version':call(path)['version'],'ended':True},key=key())
 call(path+'/followups','POST',{'expected_version':call(path)['version'],'task':{'title':'Archived denied'}},expected=409,key=key())
 call(link_path,'PATCH',{'expected_version':call(path)['version'],'ended':False},expected=409,key=key())
 call(path+'/archive','POST',{'expected_version':call(path)['version'],'archived':False},key=key())
 call(link_path,'PATCH',{'expected_version':call(path)['version'],'ended':False},key=key())
 # A real foreign task cannot be linked even by a broad workspace reader.
 foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('CRM task foreign') RETURNING id").splitlines()[0]
 foreign_member=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Foreign task owner') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',id FROM u RETURNING id").splitlines()[0]
 foreign_task=f.sql("INSERT INTO tasks.tasks(workspace_id,title,assigned_membership_id) VALUES('"+foreign+"','Foreign task','"+foreign_member+"') RETURNING id").splitlines()[0]
 call(path+'/tasks','POST',{'expected_version':call(path)['version'],'task_id':foreign_task},expected=404,key=key())
 f.sql("DO $$ BEGIN BEGIN INSERT INTO crm."+singular+"_tasks(workspace_id,"+singular+"_id,task_id,created_by_membership_id) VALUES('"+f.workspace+"','"+record['id']+"','"+foreign_task+"','"+other+"'); RAISE EXCEPTION 'Foreign task linked'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
 assert f.cli(['crm',singular+'-task-history',record['id'],link_id])['link']['id']==link_id
 result=f.mcp('crm_create_'+singular+'_followup',{'workspace_id':f.workspace,'id':record['id'],'idempotency_key':key(),'change':{'expected_version':call(path)['version'],'task':{'title':name+' MCP'}}});assert not result.get('isError'),result
 module=next(m for m in call(base+'/modules') if m['key']=='crm');module=call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':False},key=key())
 try:
  call(path+'/tasks',expected=403);call(path+'/followups','POST',change,expected=403,key=retry)
  assert call(base+'/tasks/'+task['id'])['id']==task['id']
 finally:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
 print('PASS: '+kind+' independent scopes, hidden pagination/replay, archive retention, atomic failure, workspace constraints and CLI/MCP')
 # Server-rendered create/link/end preserve rejected data, expected versions and keys.
 from browser_forms import Forms
 from urllib.request import Request
 from urllib.error import HTTPError
 from urllib.parse import urlencode
 def browser(path,body=None,expected=200):
  request=Request(f.base+path,data=None if body is None else urlencode(body,doseq=True).encode(),headers={'Origin':f.base,'Accept':'text/html'})
  try:response=f.browser.open(request,timeout=20)
  except HTTPError as failure:response=failure
  with response:
   html=response.read().decode();assert response.status==expected,(path,response.status,html[:200]);return Forms(html),html
 web='/workspaces/'+f.workspace+'/crm/'+kind+'/'+record['id']+'/tasks'
 forms,html=browser(web);draft=forms.form('title');draft.update({'title':name+' browser','notes':'Retained follow-up notes','status':'unsupported','due_on':'2028-05-01'})
 forms,html=browser(web+'?/create',draft,400);retained=forms.form('title');assert all(retained[field]==draft[field] for field in draft)
 retained['status']='open';browser(web+'?/create',retained)
 browser_task=call(base+'/tasks?search='+name.replace(' ','%20')+'%20browser')['items'][0]
 # Existing-task selection remains in the form even outside the first 100 choices.
 offpage=create('/tasks',{'title':name+' off-page browser'})
 forms,_=browser(web);draft=forms.form('task_id');draft['task_id']=offpage['id']
 call(path,'PATCH',{**body,'expected_version':call(path)['version']},key=key())
 forms,html=browser(web+'?/link',draft,409);retained=forms.form('task_id');assert all(retained[field]==draft[field] for field in draft);assert 'Discard draft and reload saved values' in html
 forms,_=browser(web);draft=forms.form('task_id');draft['task_id']=offpage['id'];browser(web+'?/link',draft)
 # Search-limited list is independent of the linked task collection.
 forms,_=browser(web);end_form=next(form['values'] for form in forms.forms if form['values'].get('link_id')==link_id)
 call(path,'PATCH',{**body,'expected_version':call(path)['version']},key=key())
 forms,html=browser(web+'?/end',end_form,409);retained=next(form['values'] for form in forms.forms if form['values'].get('link_id')==link_id);assert all(retained[field]==end_form[field] for field in end_form)
 forms,_=browser(web);end_form=next(form['values'] for form in forms.forms if form['values'].get('link_id')==link_id)
 browser(web+'?/end',{**end_form,'ended':'invalid'},400);browser(web+'?/end',end_form)
 _,html=browser(web+'/'+link_id);assert 'Link ended' in html and task['title'] in html
 assert call(base+'/tasks/'+browser_task['id'])['notes']=='Retained follow-up notes'
 print('PASS: '+kind+' browser follow-up creation, off-page selection, retained link/end conflicts and link history')
 # Same-key overlap creates one task; conflicting independent edits cannot create orphans.
 import json
 from concurrent.futures import ThreadPoolExecutor
 from urllib.request import urlopen
 for same_key in [True,False]:
  change={'expected_version':call(path)['version'],'task':{'title':name+' race'}};shared_key=key()
  before_count=int(f.sql("SELECT count(*) FROM tasks.tasks WHERE workspace_id='"+f.workspace+"'"))
  def race(number):
   request=Request(f.base+path+'/followups',method='POST',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':shared_key if same_key else key()},data=json.dumps(change).encode())
   try:response=urlopen(request,timeout=20)
   except HTTPError as failure:response=failure
   with response:return response.status,json.loads(response.read())
  with ThreadPoolExecutor(max_workers=2) as pool:results=list(pool.map(race,[1,2]))
  assert sorted(status for status,_ in results)==([201,201] if same_key else [201,409]),results
  if same_key:assert results[0][1]==results[1][1]
  assert int(f.sql("SELECT count(*) FROM tasks.tasks WHERE workspace_id='"+f.workspace+"'"))==before_count+1
 print('PASS: '+kind+' overlapping retry and optimistic follow-up creation leave exactly one task')

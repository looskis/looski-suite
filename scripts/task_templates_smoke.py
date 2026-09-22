#!/usr/bin/env python3
"""Shared task configuration and immutable reusable work specifications."""
import secrets
from integration_fixture import Fixture
f=Fixture();call=f.call;base='/api/v1/workspaces/'+f.workspace
name='Task template '+secrets.token_hex(5)
def key():return secrets.token_hex(16)
def create(path,body):return call(base+path,'POST',body,expected=201,key=key())
type_definition=create('/tasks/types',{'name':name+' follow-up','position':10})
fields={'title':name+' call customer','notes':'Review the agreed requirements','type_id':type_definition['id'],'assignment':'initiator','due_offset_days':2}
request={'name':name,**fields}
retry=key();template=call(base+'/tasks/templates','POST',request,expected=201,key=retry)
assert call(base+'/tasks/templates','POST',request,expected=201,key=retry)==template
assert template['revision']['title']==fields['title'] and template['revision']['number']==1
path=base+'/tasks/templates/'+template['id']
changed=call(path,'PATCH',{**request,'title':name+' revised task','expected_version':template['version']},key=key())
assert changed['revision']['number']==2 and changed['revision']['id']!=template['revision']['id']
assert call(path+'/revisions')['items'][0]==template['revision']
print('PASS: shared task definitions, immutable revisions and stable retries')
# A pinned older revision produces exactly its work; rename/retirement never edits instances.
instantiate={'revision_id':template['revision']['id'],'base_on':'2028-02-28'}
instance_key=key();task=call(path+'/instantiate','POST',instantiate,expected=201,key=instance_key)
assert task['title']==fields['title'] and task['due_on']=='2028-03-01' and task['type_id']==type_definition['id']
assert task['source_template_revision_id']==template['revision']['id']
assert call(path+'/instantiate','POST',instantiate,expected=201,key=instance_key)==task
assert call(base+'/tasks/'+task['id']+'/history')['items'][0]['snapshot']['source_template_revision_id']==template['revision']['id']
metadata=call(path,'PATCH',{**request,'title':changed['revision']['title'],'name':name+' renamed','expected_version':changed['version']},key=key())
assert metadata['revision']==changed['revision']
assert call(base+'/tasks/'+task['id'])==task
retired=call(path,'PATCH',{**request,'title':changed['revision']['title'],'archived':True,'expected_version':metadata['version']},key=key())
call(path+'/instantiate','POST',instantiate,expected=409,key=key())
assert call(path+'/instantiate','POST',instantiate,expected=201,key=instance_key)==task
assert call(base+'/tasks/'+task['id'])==task and call(path+'/revisions')['items'][0]==template['revision']
active=call(path,'PATCH',{**request,'title':changed['revision']['title'],'archived':False,'expected_version':retired['version']},key=key())
# Renamed/retired task type snapshots and omitted versus cleared assignment.
type_path=base+'/tasks/types/'+type_definition['id']
renamed=call(type_path,'PATCH',{'name':name+' renamed type','expected_version':1},key=key())
assert call(base+'/tasks/'+task['id'])['type_name']==type_definition['name']
pinned_after_rename=call(path+'/instantiate','POST',instantiate,expected=201,key=key())
assert pinned_after_rename['type_name']==type_definition['name']
assert call(base+'/tasks/'+pinned_after_rename['id']+'/history')['items'][0]['snapshot']['type_name']==type_definition['name']
current=call(base+'/tasks/'+task['id']);task_fields={k:current[k] for k in ['title','notes','due_on','status']}
updated=call(base+'/tasks/'+task['id'],'PATCH',{**task_fields,'expected_version':current['version']},key=key())
assert updated['type_id']==type_definition['id'] and updated['type_name']==renamed['name']
call(type_path,'PATCH',{'name':renamed['name'],'archived':True,'expected_version':renamed['version']},key=key())
call(path+'/instantiate','POST',instantiate,expected=409,key=key())
updated=call(base+'/tasks/'+task['id'],'PATCH',{**task_fields,'expected_version':updated['version']},key=key())
assert updated['type_id']==type_definition['id']
updated=call(base+'/tasks/'+task['id'],'PATCH',{**task_fields,'type_id':None,'expected_version':updated['version']},key=key())
assert updated['type_id'] is None and updated['type_name'] is None
assert updated['source_template_revision_id']==template['revision']['id']
call(base+'/tasks','POST',{'title':'Untrusted source','source_template_revision_id':template['revision']['id']},expected=400,key=key())
call(base+'/tasks','POST',{'title':'Retired type denied','type_id':type_definition['id']},expected=409,key=key())
call(type_path,'PATCH',{'name':renamed['name'],'archived':False,'expected_version':3},key=key())
# Validation applies to fixed membership and calendar arithmetic, including leap days.
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Template assignee') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
def grant(action,scope):f.sql("UPDATE suite.role_permissions SET record_scope='"+scope+"' WHERE role_id='"+role+"' AND permission_key='"+action+"'")
for change in [{'assignment':'fixed'},{'fixed_membership_id':other},{'due_offset_days':3651}]:
 call(base+'/tasks/templates','POST',{**request,**change},expected=400,key=key())
fixed=create('/tasks/templates',{**request,'assignment':'fixed','fixed_membership_id':other})
context=create('/tasks/templates',{**request,'assignment':'context_owner','due_offset_days':-1})
fixed_input={'revision_id':fixed['revision']['id'],'base_on':'2028-02-28'}
context_input={'revision_id':context['revision']['id'],'base_on':'2028-03-01'}
call(base+'/tasks/templates/'+context['id']+'/instantiate','POST',context_input,expected=400,key=key())
context_task=call(base+'/tasks/templates/'+context['id']+'/instantiate','POST',{**context_input,'context_owner_membership_id':other},expected=201,key=key())
assert context_task['due_on']=='2028-02-29' and context_task['assigned_membership_id']==other
call(path+'/instantiate','POST',{**instantiate,'base_on':'9999-12-31'},expected=400,key=key())
call(path+'/instantiate','POST',{**instantiate,'base_on':'2028-02-30'},expected=400,key=key())
call(path+'/instantiate','POST',fixed_input,expected=404,key=key())
f.sql("UPDATE suite.memberships SET state='suspended' WHERE id='"+other+"'")
try:call(base+'/tasks/templates/'+fixed['id']+'/instantiate','POST',fixed_input,expected=400,key=key())
finally:f.sql("UPDATE suite.memberships SET state='active' WHERE id='"+other+"'")
grant('tasks.create','assigned')
try:
 call(base+'/tasks/templates/'+fixed['id']+'/instantiate','POST',fixed_input,expected=403,key=key())
 call(base+'/tasks/templates/'+context['id']+'/instantiate','POST',{**context_input,'context_owner_membership_id':other},expected=403,key=key())
finally:grant('tasks.create','workspace')
# Retry authority follows the current task assignee.
current=call(base+'/tasks/'+task['id']);call(base+'/tasks/'+task['id'],'PATCH',{**task_fields,'assigned_membership_id':other,'expected_version':current['version']},key=key())
grant('tasks.create','assigned')
try:call(path+'/instantiate','POST',instantiate,expected=404,key=instance_key)
finally:grant('tasks.create','workspace')
# Both revision publication and instantiated work roll back with failed audit/history.
before=call(path);before_audit=f.sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"'")
f.sql('REVOKE INSERT ON tasks.template_revisions FROM looski_runtime');failed_key=key();pending={**request,'title':name+' atomic revision','expected_version':before['version']}
try:call(path,'PATCH',pending,expected=503,key=failed_key)
finally:f.sql('GRANT INSERT ON tasks.template_revisions TO looski_runtime')
assert call(path)==before and f.sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"'")==before_audit
published=call(path,'PATCH',pending,key=failed_key);assert call(path,'PATCH',pending,key=failed_key)==published
before_tasks=f.sql("SELECT count(*) FROM tasks.tasks WHERE workspace_id='"+f.workspace+"'");before_audit=f.sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"'")
f.sql('REVOKE INSERT ON tasks.task_changes FROM looski_runtime');failed_key=key()
try:call(path+'/instantiate','POST',instantiate,expected=503,key=failed_key)
finally:f.sql('GRANT INSERT ON tasks.task_changes TO looski_runtime')
assert f.sql("SELECT count(*) FROM tasks.tasks WHERE workspace_id='"+f.workspace+"'")==before_tasks and f.sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"'")==before_audit
created=call(path+'/instantiate','POST',instantiate,expected=201,key=failed_key)
assert call(path+'/instantiate','POST',instantiate,expected=201,key=failed_key)==created
print('PASS: pinned task creation, type snapshots, archive retention, date/assignment scope and atomic revision/instance failure')
# Configuration grants are independent of Tasks grants, and shared templates survive CRM disablement.
f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='tasks.configuration.read'")
try:
 call(path,expected=403);call(path+'/revisions',expected=403);call(path+'/instantiate','POST',instantiate,expected=403,key=instance_key)
 call(base+'/tasks/'+created['id'])
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','tasks.configuration.read')")
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if module['enabled']:call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key())
try:
 assert call(path);assert call(type_path);assert call(path+'/instantiate','POST',instantiate,expected=201,key=key())
finally:
 if module['enabled']:
  current_module=next(m for m in call(base+'/modules') if m['key']=='crm');call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':current_module['version']},key=key())
foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('Task config foreign') RETURNING id").splitlines()[0]
foreign_type=f.sql("INSERT INTO tasks.task_types(workspace_id,name) VALUES('"+foreign+"','Foreign type') RETURNING id").splitlines()[0]
call(base+'/tasks','POST',{'title':'Wrong workspace type','type_id':foreign_type},expected=404,key=key())
call(base+'/tasks/templates','POST',{**request,'type_id':foreign_type},expected=404,key=key())
f.sql("DO $$ BEGIN BEGIN UPDATE tasks.tasks SET type_id='"+foreign_type+"',type_name='Foreign' WHERE id='"+created['id']+"'; RAISE EXCEPTION 'Foreign type accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
# Final authority is current on both read and creation replay after reassignment.
grant('tasks.read','assigned')
try:call(path+'/instantiate','POST',instantiate,expected=404,key=instance_key)
finally:grant('tasks.read','workspace')
assert f.cli(['tasks','template-revisions',template['id'],'--limit','1'])['items'][0]==template['revision']
assert not f.mcp('tasks_template',{'workspace_id':f.workspace,'id':template['id']}).get('isError')
# Concurrent publication accepts one expected version; a duplicated creation key creates one task.
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request,urlopen
from urllib.error import HTTPError
import json
def parallel_request(route,method,payload,k):
 req=Request(f.base+route,method=method,headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':k},data=json.dumps(payload).encode())
 try:r=urlopen(req,timeout=20)
 except HTTPError as e:r=e
 with r:return r.status,json.loads(r.read())
current=call(path);pending={**request,'title':name+' concurrent','expected_version':current['version']}
with ThreadPoolExecutor(2) as executor:replies=list(executor.map(lambda k:parallel_request(path,'PATCH',pending,k),[key(),key()]))
assert sorted(r[0] for r in replies)==[200,409],replies
assert call(path)['revision']['number']==current['revision']['number']+1
same=key();before=f.sql("SELECT count(*) FROM tasks.tasks WHERE workspace_id='"+f.workspace+"'")
with ThreadPoolExecutor(2) as executor:replies=list(executor.map(lambda k:parallel_request(path+'/instantiate','POST',instantiate,k),[same,same]))
assert [r[0] for r in replies]==[201,201] and replies[0][1]==replies[1][1]
assert int(f.sql("SELECT count(*) FROM tasks.tasks WHERE workspace_id='"+f.workspace+"'"))==int(before)+1
page=call(path+'/revisions?limit=1');assert page['next_cursor']==template['revision']['id']
assert call(path+'/revisions?limit=1&after='+page['next_cursor'])['items'][0]==changed['revision']
call(path+'/revisions?after='+fixed['revision']['id'],expected=404)
print('PASS: independent grants/module availability, workspace constraints, revision pagination, CLI/MCP and concurrent publication/creation')
# Browser template editing and execution retain drafts and original retry identities.
from browser_forms import Forms
from urllib.parse import urlencode,urlparse

def browser(route,values=None,expected=200):
 req=Request(f.base+route,data=None if values is None else urlencode(values).encode(),headers={'Origin':f.base,'Accept':'text/html'})
 try:response=f.browser.open(req,timeout=20)
 except HTTPError as e:response=e
 with response:
  html=response.read().decode();assert response.status==expected,(route,response.status,expected,html[:250]);return Forms(html),html,response.url
web='/workspaces/'+f.workspace+'/tasks'
forms,_,_=browser(web+'/types');draft=forms.form('name');draft['name']=name+' browser type';forms,_,url=browser(web+'/types',draft);browser_type=urlparse(url).path.rsplit('/',1)[1]
forms,_,_=browser(web+'/templates');draft=forms.form('title');draft.update(name=name+' browser template',title=name+' browser task',type_id=browser_type,assignment='invalid')
forms,_,_=browser(web+'/templates',draft,400);assert forms.form('title')==draft
draft['assignment']='initiator';forms,_,url=browser(web+'/templates',draft);browser_template=urlparse(url).path.rsplit('/',1)[1];browser_path=web+'/templates/'+browser_template
stale=forms.form('title');stale['title']=name+' stale template';current=call(base+'/tasks/templates/'+browser_template)
call(base+'/tasks/templates/'+browser_template,'PATCH',{'name':current['name'],**{k:current['revision'][k] for k in ['title','notes','type_id','assignment','fixed_membership_id','due_offset_days']},'title':name+' server revised','expected_version':current['version']},key=key())
forms,html,_=browser(browser_path+'?/save',stale,409);assert forms.form('title')==stale and 'Discard draft' in html
forms,_,_=browser(browser_path);execution=forms.form('base_on');execution['base_on']='2028-02-30'
forms,_,_=browser(browser_path+'?/instantiate',execution,400);assert forms.form('base_on')==execution
execution['base_on']='2028-02-29';forms,html,url=browser(browser_path+'?/instantiate',execution);browser_task=urlparse(url).path.rsplit('/',1)[1]
assert call(base+'/tasks/'+browser_task)['source_template_revision_id']==execution['revision_id']
assert 'Created from template revision' in html
fields_form=forms.form('title');assert fields_form['type_id']==browser_type
fields_form['type_id']='';browser(web+'/'+browser_task+'?/save',fields_form)
assert call(base+'/tasks/'+browser_task)['type_id'] is None
print('PASS: browser type/template creation, retained invalid/conflicting drafts, pinned execution and explicit task type clearing')

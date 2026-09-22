#!/usr/bin/env python3
"""Shared Tasks behavioral acceptance on an isolated installation."""
import secrets
from integration_fixture import Fixture
f=Fixture(); call=f.call; base='/api/v1/workspaces/'+f.workspace
name='Task '+secrets.token_hex(5)
def key(): return secrets.token_hex(16)
retry=key(); body={'title':name,'notes':'Follow up','due_on':'2028-02-29','status':'open'}
task=call(base+'/tasks','POST',body,expected=201,key=retry)
assert call(base+'/tasks','POST',body,expected=201,key=retry)==task
assert task['version']==1 and task['completed_at'] is None and not task['archived']
assert call(base+'/tasks/'+task['id'])==task
print('PASS: shared task creation, read and original-result retry')
path=base+'/tasks/'+task['id']
def edit(record,**changes):
 payload={field:record[field] for field in ['title','notes','assigned_membership_id','due_on','status']}
 return call(base+'/tasks/'+record['id'],'PATCH',{**payload,**changes,'expected_version':record['version']},key=key())
first=task
for status in ['in_progress','completed']:
 task=edit(task,status=status)
completed_at=task['completed_at']; assert completed_at
completed=task
task=edit(task,notes='Completed details'); assert task['completed_at']==completed_at
task=edit(task,status='open'); assert task['completed_at'] is None
task=edit(task,status='cancelled'); assert task['completed_at'] is None
history=call(path+'/history')['items'];assert len(history)==6 and history[0]['snapshot']==first
assert history[2]['snapshot']==completed
page=call(path+'/history?limit=2');assert len(page['items'])==2 and page['next_cursor']
page2=call(path+'/history?limit=2&after='+page['next_cursor']);assert len(page2['items'])==2 and page2['items'][0]['snapshot']==completed
call(path+'/history?after='+task['id'],expected=404)
# Invalid dates/statuses and stale input do not write or consume a key.
for value in ['2027-02-29','2026-04-31','0000-01-01','2026-1-01','2026-01-00','infinity']:
 call(base+'/tasks','POST',{**body,'due_on':value},expected=400,key=key())
call(base+'/tasks','POST',{**body,'status':'other'},expected=400,key=key())
call(path,'PATCH',{**body,'expected_version':1},expected=409,key=key())
# Archive preserves completion/history and rejects edits until restored.
task=call(path+'/archive','POST',{'expected_version':task['version'],'archived':True},key=key())
call(path,'PATCH',{**body,'expected_version':task['version']},expected=409,key=key())
assert task['id'] in [t['id'] for t in call(base+'/tasks?archived=true&search='+name.replace(' ','%20'))['items']]
task=call(path+'/archive','POST',{'expected_version':task['version'],'archived':False},key=key())
assert len(call(path+'/history')['items'])==8
assert f.sql("SELECT details->>'archived' FROM suite.audit_events WHERE workspace_id='"+f.workspace+"' AND record_id='"+task['id']+"' AND action='tasks.archived' ORDER BY occurred_at DESC LIMIT 1")== 'false'
print('PASS: task lifecycle, completion timestamps, immutable snapshots, archive/restore and validation')
# Per-action scope intersections, saved replay revocation and assignment constraints.
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Task assignee fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
hidden=call(base+'/tasks','POST',{**body,'title':name+' hidden','assigned_membership_id':other},expected=201,key=key());hidden_path=base+'/tasks/'+hidden['id']
def grant(action,scope):f.sql("UPDATE suite.role_permissions SET record_scope='"+scope+"' WHERE role_id='"+role+"' AND permission_key='"+action+"'")
grant('tasks.update','assigned')
try:
 call(hidden_path,'PATCH',{**body,'expected_version':1},expected=404,key=key())
 call(path,'PATCH',{**body,'assigned_membership_id':other,'expected_version':task['version']},expected=403,key=key())
finally:grant('tasks.update','workspace')
grant('tasks.read','assigned')
try:
 call(hidden_path,expected=404);call(hidden_path+'/history',expected=404)
 call(base+'/tasks?after='+hidden['id'],expected=404)
 assert call(base+'/tasks?assigned_membership_id='+other)['items']==[]
 assert all(r['id']!=hidden['id'] for r in call(base+'/tasks?search='+name.replace(' ','%20'))['items'])
 call(path,'PATCH',{**body,'assigned_membership_id':other,'expected_version':task['version']},expected=403,key=key())
finally:grant('tasks.read','workspace')
# Revoking creation prevents original-result replay immediately.
f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='tasks.create'")
try:call(base+'/tasks','POST',body,expected=403,key=retry)
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','tasks.create')")
# Blank assignment in edits preserves current assignee; explicit reassignment works.
hidden=edit(hidden,assigned_membership_id=None);assert hidden['assigned_membership_id']==other
hidden=edit(hidden,assigned_membership_id=task['assigned_membership_id']);assert hidden['assigned_membership_id']==task['assigned_membership_id']
f.sql("UPDATE suite.memberships SET state='suspended' WHERE id='"+other+"'")
try:call(base+'/tasks','POST',{**body,'assigned_membership_id':other},expected=400,key=key())
finally:f.sql("UPDATE suite.memberships SET state='active' WHERE id='"+other+"'")
foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('Task foreign fixture') RETURNING id").splitlines()[0]
foreign_member=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Task foreign member') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',id FROM u RETURNING id").splitlines()[0]
foreign_task=f.sql("INSERT INTO tasks.tasks(workspace_id,title,assigned_membership_id) VALUES('"+foreign+"','Foreign task','"+foreign_member+"') RETURNING id").splitlines()[0]
call(base+'/tasks/'+foreign_task,expected=404)
call(base+'/tasks','POST',{**body,'assigned_membership_id':foreign_member},expected=400,key=key())
f.sql("DO $$ BEGIN BEGIN INSERT INTO tasks.tasks(workspace_id,title,assigned_membership_id) VALUES('"+f.workspace+"','Bad foreign task','"+foreign_member+"'); RAISE EXCEPTION 'foreign assignment accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
print('PASS: task read/write scope intersection, immediate replay revocation, reassignment, inactive and foreign memberships')
# Independent shared capability stays usable with optional CRM disabled.
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if module['enabled']:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':False},key=key())
try:
 task=edit(call(path),notes='Shared Tasks works independently')
 assert call(path)['notes']==task['notes']
finally:
 if module['enabled']:
  current=next(m for m in call(base+'/modules') if m['key']=='crm');call(base+'/modules/crm','PATCH',{'expected_version':current['version'],'enabled':True},key=key())
# Failed audit rolls task, history and replay back, same key can then succeed.
failed_key=key();change={**body,'title':name+' atomic','expected_version':task['version']};before_history=call(path+'/history')
f.sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:call(path,'PATCH',change,expected=503,key=failed_key)
finally:f.sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert call(path)==task and call(path+'/history')==before_history
task=call(path,'PATCH',change,key=failed_key)
assert call(path,'PATCH',change,key=failed_key)==task
call(path,'PATCH',{**change,'notes':'different'},expected=409,key=failed_key)
# Simultaneous edits use optimistic conflict protection.
import json
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request,urlopen
from urllib.error import HTTPError
from urllib.parse import urlencode
version=task['version']
def race(title):
 request=Request(f.base+path,method='PATCH',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':key()},data=json.dumps({**body,'title':title,'expected_version':version}).encode())
 try:response=urlopen(request,timeout=20)
 except HTTPError as failure:response=failure
 with response:return response.status
with ThreadPoolExecutor(max_workers=2) as pool:assert sorted(pool.map(race,[name+' concurrent A',name+' concurrent B']))==[200,409]
task=call(path)
assert f.cli(['tasks','get',task['id']])==task
result=f.mcp('tasks_edit',{'workspace_id':f.workspace,'id':task['id'],'idempotency_key':key(),'change':{**body,'notes':'MCP parity','expected_version':task['version']}});assert not result.get('isError'),result
assert call(path)['notes']=='MCP parity'
assert f.cli(['tasks','list','--due-before','2028-02-29','--search',name])['items']
assert f.cli(['tasks','history',task['id'],'--limit','2'])['next_cursor']
print('PASS: task module independence, audit/history/replay rollback, concurrent edits, filters and CLI/MCP parity')
# SSR drafts and conflict recovery preserve original input/version/key.
from browser_forms import Forms
def browser(path,body=None,expected=200):
 request=Request(f.base+path,data=None if body is None else urlencode(body,doseq=True).encode(),headers={'Origin':f.base,'Accept':'text/html'})
 try:response=f.browser.open(request,timeout=20)
 except HTTPError as failure:response=failure
 with response:
  html=response.read().decode();assert response.status==expected,(path,response.status,html[:200]);return Forms(html),html,response.url
web='/workspaces/'+f.workspace+'/tasks'
forms,_,_=browser(web);draft=forms.form('title');draft.update({'title':name+' browser','notes':'Kept notes','status':'unsupported','due_on':'2028-02-29','assigned_membership_id':other})
forms,html,_=browser(web,draft,400);retained=forms.form('title');assert all(retained[field]==draft[field] for field in draft)
retained['status']='open';forms,html,url=browser(web,retained);browser_id=url.rsplit('/',1)[1]
assert call(base+'/tasks/'+browser_id)['assigned_membership_id']==other
browser_path=web+'/'+browser_id
forms,html,_=browser(browser_path);assert 'keep the current assignee' in html
stale=forms.form('title');stale['title']=name+' stale';stale['assigned_membership_id']=''
current=call(base+'/tasks/'+browser_id);edit(current,title=name+' server')
forms,html,_=browser(browser_path+'?/save',stale,409);retained=forms.form('title');assert all(retained[field]==stale[field] for field in stale)
assert 'Discard draft and reload saved values' in html
forms,_,_=browser(browser_path);fresh=forms.form('title');fresh['assigned_membership_id']='';browser(browser_path+'?/save',fresh)
assert call(base+'/tasks/'+browser_id)['assigned_membership_id']==other
_,html,_=browser(web+'?assigned_membership_id='+other+'&due_before=2028-02-29');assert browser_id in html and task['id'] not in html
forms,_,_=browser(browser_path);archive=next(form['values'] for form in forms.forms if form['action']=='?/archive')
browser(browser_path+'?/archive',{**archive,'archived':'invalid'},400)
browser(browser_path+'?/archive',archive)
assert call(base+'/tasks/'+browser_id)['archived']
print('PASS: browser task creation, unsupported input, retained stale drafts, assignment semantics, filters and archive')

#!/usr/bin/env python3
"""A combined timeline composes independently authorized history sources."""
import secrets,json
from urllib.parse import urlencode
from urllib.request import Request
from integration_fixture import Fixture
f=Fixture();call=f.call;base='/api/v1/workspaces/'+f.workspace
name='Timeline '+secrets.token_hex(6)
def key():return secrets.token_hex(16)
def create(path,body):return call(base+path,'POST',body,expected=201,key=key())
def timeline(kind,id,**kwargs):return base+'/crm/timeline?'+urlencode({'anchor_kind':kind,'anchor_id':id,**kwargs})
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if not module['enabled']:call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key())
contact=create('/contacts',{'display_name':name})
assert call(timeline('contact',contact['id']))['items']==[]
type=create('/crm/activity-types',{'name':name,'behavior':'note'})
activity_input={'type_id':type['id'],'subject':name,'body':'Original evidence','occurred_at':'2028-01-01T00:00:00Z'}
a=create('/crm/activities',{'anchor_kind':'contact','anchor_id':contact['id'],**activity_input})
call(base+'/crm/activities/'+a['id'],'PATCH',{**activity_input,'body':'Revised evidence','expected_version':1},key=key())
call(base+'/crm/activities/'+a['id']+'/archive','POST',{'archived':True,'expected_version':2},key=key())
page=call(timeline('contact',contact['id']));assert [e['event']['detail']['snapshot']['version'] for e in page['items']]==[3,2,1]
assert all(e['event']['kind']=='activity' for e in page['items'])
assert page['items'][-1]['event']['detail']['snapshot']['body']=='Original evidence'
assert page['items'][0]['event']['detail']['snapshot']['archived']
print('PASS: contact activity creation/edit/archive history and original snapshots')
pipeline=create('/crm/pipelines',{'name':name});stage=create('/crm/stages',{'name':name,'parent_id':pipeline['id'],'outcome':'open'})
won=create('/crm/stages',{'name':'Accepted','parent_id':pipeline['id'],'outcome':'won'})
opp=create('/crm/opportunities',{'title':name,'pipeline_id':pipeline['id'],'stage_id':stage['id']})
call(base+'/crm/opportunities/'+opp['id'],'PATCH',{'title':name,'pipeline_id':pipeline['id'],'stage_id':won['id'],'expected_version':1},key=key())
assert len(call(timeline('opportunity',opp['id']))['items'])==2
category=create('/crm/categories',{'name':name});status=create('/crm/statuses',{'name':'Planned','parent_id':category['id']});active=create('/crm/statuses',{'name':'Active','parent_id':category['id']})
deal=create('/crm/deals',{'title':name,'category_id':category['id'],'status_id':status['id']})
dp=base+'/crm/deals/'+deal['id']
call(dp,'PATCH',{'title':name,'category_id':category['id'],'status_id':active['id'],'expected_version':1},key=key())
call(base+'/crm/statuses/'+active['id'],'PATCH',{'name':'Renamed Active','parent_id':category['id'],'expected_version':1},key=key())
activity=create('/crm/activities',{'anchor_kind':'deal','anchor_id':deal['id'],**activity_input})
link=call(dp+'/followups','POST',{'expected_version':call(dp)['version'],'task':{'title':name+' follow-up','notes':'Independent task','status':'open'}},expected=201,key=key())
task=link['task'];tp=base+'/tasks/'+task['id']
call(tp,'PATCH',{'title':task['title'],'notes':task['notes'],'status':'completed','expected_version':task['version']},key=key())
for ended in [True,False]:call(dp+'/tasks/'+link['id'],'PATCH',{'ended':ended,'expected_version':call(dp)['version']},key=key())
call(tp+'/archive','POST',{'archived':True,'expected_version':2},key=key())
page=call(timeline('deal',deal['id'],limit=100));events=page['items'];kinds=[e['event']['kind'] for e in events]
assert kinds.count('transition')==2 and kinds.count('activity')==1 and kinds.count('task_link')==3 and kinds.count('task')==3,kinds
assert [e['event']['detail']['to_label'] for e in events if e['event']['kind']=='transition']==['Active','Planned']
assert [e['event']['detail']['snapshot']['status'] for e in events if e['event']['kind']=='task']==['completed','completed','open']
assert all(e['event']['detail']['snapshot']['id']==task['id'] for e in events if e['event']['kind']=='task')
collected=[];after=None
for count in range(20):
 page=call(timeline('deal',deal['id'],limit=2,**({'after':after} if after else {})));collected+=page['items'];after=page['next_cursor']
 if not after:break
assert collected==events and len(set(e['cursor'] for e in collected))==len(events)
# Equal recording instants still produce stable, duplicate-free pages across sources.
f.sql("UPDATE crm.activity_changes SET occurred_at='2029-01-01T00:00:00Z' WHERE activity_id='"+activity['id']+"'; UPDATE tasks.task_changes SET occurred_at='2029-01-01T00:00:00Z' WHERE task_id='"+task['id']+"'")
equal=call(timeline('deal',deal['id'],limit=100))['items'];collected=[];after=None
for count in range(20):
 page=call(timeline('deal',deal['id'],limit=1,**({'after':after} if after else {})));collected+=page['items'];after=page['next_cursor']
 if not after:break
assert collected==equal
call(timeline('deal',deal['id'],limit=101),expected=400);call(timeline('deal',deal['id'],after='invalid'),expected=400)
call(timeline('contact',contact['id'],after=equal[0]['cursor']),expected=404)
# Shared tasks do not make continuation cursors interchangeable between parents.
shared=create('/crm/deals',{'title':name+' shared','category_id':category['id'],'status_id':status['id']})
# The task is archived: restore before creating a new association.
current=call(tp);call(tp+'/archive','POST',{'archived':False,'expected_version':current['version']},key=key())
call(base+'/crm/deals/'+shared['id']+'/tasks','POST',{'task_id':task['id'],'expected_version':1},expected=201,key=key())
task_cursor=next(e['cursor'] for e in equal if e['event']['kind']=='task')
call(timeline('deal',shared['id'],after=task_cursor),expected=404)
# Unrelated Tasks history is never decoded while composing this parent's timeline.
unrelated=create('/tasks',{'title':name+' unrelated'})
saved=f.sql("SELECT snapshot::text FROM tasks.task_changes WHERE task_id='"+unrelated['id']+"'")
f.sql("UPDATE tasks.task_changes SET snapshot=snapshot-'title' WHERE task_id='"+unrelated['id']+"'")
try:assert call(timeline('deal',deal['id']))['items']
finally:
 literal=saved.replace("'","''")
 f.sql("UPDATE tasks.task_changes SET snapshot='"+literal+"'::jsonb WHERE task_id='"+unrelated['id']+"'")
print('PASS: interleaved activity/task/link/status events, retained labels, task identity and deterministic cursor pages including tied times')
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Timeline reader fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
def grant(permission,scope):f.sql("UPDATE suite.role_permissions SET record_scope='"+scope+"' WHERE role_id='"+role+"' AND permission_key='"+permission+"'")
for permission,hidden in [('tasks.read',['task','task_link']),('crm.activities.read',['activity'])]:
 cursor=next(e['cursor'] for e in equal if e['event']['kind']==hidden[0])
 f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='"+permission+"'")
 try:
  visible=call(timeline('deal',deal['id']))
  assert not any(e['event']['kind'] in hidden for e in visible['items'])
  assert (task['id'] if permission=='tasks.read' else activity['id']) not in json.dumps(visible)
  call(timeline('deal',deal['id'],after=cursor),expected=404)
 finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','"+permission+"')")
# Reassign a task; its historical snapshots follow current Tasks authority.
current=call(tp);call(tp+'/archive','POST',{'archived':False,'expected_version':current['version']},key=key());current=call(tp)
call(tp,'PATCH',{'title':task['title'],'notes':task['notes'],'assigned_membership_id':other,'status':'completed','expected_version':current['version']},key=key())
grant('tasks.read','assigned')
try:assert task['id'] not in json.dumps(call(timeline('deal',deal['id'])))
finally:grant('tasks.read','workspace')
hidden_deal=create('/crm/deals',{'title':name+' hidden','category_id':category['id'],'status_id':status['id'],'owner_membership_id':other})
grant('crm.deals.read','assigned')
try:call(timeline('deal',hidden_deal['id']),expected=404)
finally:grant('crm.deals.read','workspace')
# More than a candidate batch of inaccessible activities cannot truncate the page.
hidden=create('/crm/activities',{'anchor_kind':'deal','anchor_id':deal['id'],'responsible_membership_id':other,**activity_input})
f.sql("INSERT INTO crm.activity_changes(workspace_id,activity_id,snapshot,audit_event_id,occurred_at) SELECT workspace_id,activity_id,snapshot,audit_event_id,'2030-01-01T00:00:00Z'::timestamptz+g*interval '1 second' FROM crm.activity_changes CROSS JOIN generate_series(1,105) g WHERE activity_id='"+hidden['id']+"'")
grant('crm.activities.read','assigned')
try:
 visible=call(timeline('deal',deal['id'],limit=100));assert hidden['id'] not in json.dumps(visible)
 assert any(e['event']['kind']=='activity' for e in visible['items']) and len(visible['items'])>9
finally:grant('crm.activities.read','workspace')
module=next(m for m in call(base+'/modules') if m['key']=='crm');call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key())
try:call(timeline('contact',contact['id']),expected=403);call(timeline('deal',deal['id']),expected=403);assert call(tp)
finally:call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']+1},key=key())
foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('Timeline foreign') RETURNING id").splitlines()[0]
call('/api/v1/workspaces/'+foreign+'/crm/timeline?anchor_kind=deal&anchor_id='+deal['id'],expected=403)
# Contact merges preserve history through authorized aliases, with both scopes checked.
survivor=create('/contacts',{'display_name':name+' survivor'})
current=call(base+'/contacts/'+contact['id'])
call(base+'/contacts/'+survivor['id']+'/merge','POST',{'source_id':contact['id'],'source_version':current['version'],'survivor_version':survivor['version'],'resolved':{'display_name':survivor['display_name'],'notes':''}},key=key())
merged=call(timeline('contact',survivor['id']));assert len(merged['items'])==3
assert all(e['event']['detail']['snapshot']['canonical_anchor_id']==survivor['id'] for e in merged['items'])
f.sql("UPDATE contacts.contacts SET owner_membership_id='"+other+"' WHERE id='"+contact['id']+"'")
grant('contacts.read','assigned')
try:
 assert call(timeline('contact',survivor['id']))['items']==[]
 call(timeline('contact',survivor['id'],after=merged['items'][0]['cursor']),expected=404)
finally:grant('contacts.read','workspace')
print('PASS: independent current source/parent scopes, cursor revocation, hidden batches, module and foreign-workspace denial')
# More than one batch of linked IDs composes a complete, globally ordered history.
bulk=create('/crm/deals',{'title':name+' many tasks','category_id':category['id'],'status_id':status['id']})
version=bulk['version'];bulk_tasks=set()
for i in range(101):
 t=create('/tasks',{'title':name+' batch '+str(i)})
 call(base+'/crm/deals/'+bulk['id']+'/tasks','POST',{'expected_version':version,'task_id':t['id']},expected=201,key=key());version+=1;bulk_tasks.add(t['id'])
seen=[];after=None
for page_number in range(20):
 page=call(timeline('deal',bulk['id'],limit=17,**({'after':after} if after else {})));seen+=page['items'];after=page['next_cursor']
 if not after:break
assert after is None and len(seen)==203 and len({e['cursor'] for e in seen})==203
assert {e['event']['detail']['snapshot']['id'] for e in seen if e['event']['kind']=='task'}==bulk_tasks
assert len([e for e in seen if e['event']['kind']=='task_link'])==101
assert seen==sorted(seen,key=lambda e:(e['recorded_at'],e['event']['kind'],e['cursor'].rsplit(':',1)[-1]),reverse=True)
print('PASS: >100 linked-task reference batches, merged boundary filtering and complete duplicate-free pagination')
assert f.cli(['crm','timeline','--anchor-kind','contact','--anchor-id',contact['id']])['items']==call(timeline('contact',contact['id']))['items']
result=f.mcp('crm_timeline',{'workspace_id':f.workspace,'query':{'anchor_kind':'contact','anchor_id':contact['id']}})
assert not result.get('isError') and json.loads(result['content'][0]['text'])['items']==call(timeline('contact',contact['id']))['items']
web='/workspaces/'+f.workspace+'/crm/timeline?anchor_kind=deal&anchor_id='+deal['id']
with f.browser.open(Request(f.base+web,headers={'Accept':'text/html'}),timeout=20) as response:
 html=response.read().decode();assert response.status==200 and 'Timeline' in html and 'Earlier events' in html
with f.browser.open(f.base+'/workspaces/'+f.workspace+'/contacts/'+contact['id'],timeout=20) as response:assert '/crm/timeline?' in response.read().decode()
print('PASS: API/CLI/MCP read parity and SSR timeline/navigation')

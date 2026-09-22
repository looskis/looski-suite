#!/usr/bin/env python3
"""Configurable activities retain history and independently scoped anchors."""
import secrets
from integration_fixture import Fixture
f=Fixture(); call=f.call; base='/api/v1/workspaces/'+f.workspace
name='Activity '+secrets.token_hex(5)
def key():return secrets.token_hex(16)
def create(path,body):return call(base+path,'POST',body,expected=201,key=key())
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if not module['enabled']:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
note=create('/crm/activity-types',{'name':name,'behavior':'note'})
interaction=create('/crm/activity-types',{'name':name+' call','behavior':'interaction'})
contact=create('/contacts',{'display_name':name})
fields={'subject':name,'body':'Recorded note','type_id':note['id'],'occurred_at':'2026-10-12T09:15:00-07:00'}
request={'anchor_kind':'contact','anchor_id':contact['id'],**fields}
retry=key(); activity=call(base+'/crm/activities','POST',request,expected=201,key=retry)
assert call(base+'/crm/activities','POST',request,expected=201,key=retry)==activity
assert activity['type_name']==name and activity['behavior']=='note'
assert activity['occurred_at']=='2026-10-12T16:15:00+00:00'
path=base+'/crm/activities/'+activity['id']
assert call(path)==activity
query='?anchor_kind=contact&anchor_id='+contact['id']
assert call(base+'/crm/activities'+query)['items']==[activity]
print('PASS: configurable activity creation, timestamp normalization and stable retry')
# Semantic behavior is immutable; historical labels are snapshots.
note_path=base+'/crm/activity-types/'+note['id']
renamed=call(note_path,'PATCH',{'name':name+' renamed','behavior':'note','expected_version':1},key=key())
call(note_path,'PATCH',{'name':name,'behavior':'interaction','expected_version':2},expected=409,key=key())
assert call(path)['type_name']==name
assert call(path+'/history')['items'][0]['snapshot']['type_name']==name
edited=call(path,'PATCH',{**fields,'subject':name+' edited','expected_version':1},key=key())
assert edited['type_name']==renamed['name']
assert [c['snapshot']['type_name'] for c in call(path+'/history')['items']]==[name,renamed['name']]
call(note_path,'PATCH',{'name':renamed['name'],'behavior':'note','archived':True,'expected_version':2},key=key())
call(base+'/crm/activities','POST',request,expected=409,key=key())
edited=call(path,'PATCH',{**fields,'expected_version':edited['version']},key=key())
for timestamp in ['2026-02-30T12:00:00Z','2026-10-12T09:15:00','infinity','2026-10-12','2016-12-31T23:59:60Z']:
 call(base+'/crm/activities','POST',{**request,'type_id':interaction['id'],'occurred_at':timestamp},expected=400,key=key())
call(path,'PATCH',{**fields,'duration_minutes':5,'expected_version':edited['version']},expected=400,key=key())
logged=create('/crm/activities',{**request,'type_id':interaction['id'],'duration_minutes':20})
assert logged['behavior']=='interaction' and logged['duration_minutes']==20
call(path+'/history?after='+logged['id'],expected=404)
# Activity revisions, audit and retry are one atomic write.
before_audit=f.sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"'")
f.sql('REVOKE INSERT ON crm.activity_changes FROM looski_runtime');failed_key=key();change={**fields,'subject':name+' rollback','expected_version':edited['version']}
try:call(path,'PATCH',change,expected=503,key=failed_key)
finally:f.sql('GRANT INSERT ON crm.activity_changes TO looski_runtime')
assert call(path)==edited and f.sql("SELECT count(*) FROM suite.audit_events WHERE workspace_id='"+f.workspace+"'")==before_audit
edited=call(path,'PATCH',change,key=failed_key);assert call(path,'PATCH',change,key=failed_key)==edited
# Real independent responsible-member and anchor-owner policy intersections.
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Activity staff') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
def grant(action,scope):f.sql("UPDATE suite.role_permissions SET record_scope='"+scope+"' WHERE role_id='"+role+"' AND permission_key='"+action+"'")
foreign_owned=create('/contacts',{'display_name':name+' restricted','owner_membership_id':other})
hidden=create('/crm/activities',{**request,'anchor_id':foreign_owned['id'],'type_id':interaction['id']})
grant('contacts.read','assigned')
try:
 call(base+'/crm/activities/'+hidden['id'],expected=404)
 call(base+'/crm/activities/'+hidden['id']+'/history',expected=404)
 call(base+'/crm/activities?anchor_kind=contact&anchor_id='+foreign_owned['id'],expected=404)
finally:grant('contacts.read','workspace')
reassigned=call(path,'PATCH',{**fields,'responsible_membership_id':other,'expected_version':edited['version']},key=key())
grant('crm.activities.read','assigned')
try:
 assert all(i['id']!=activity['id'] for i in call(base+'/crm/activities'+query)['items'])
 call(path,expected=404);call(path+'/history',expected=404)
 call(base+'/crm/activities'+query+'&after='+activity['id'],expected=404)
 call(base+'/crm/activities','POST',request,expected=404,key=retry)
finally:grant('crm.activities.read','workspace')
grant('crm.activities.update','assigned')
try:call(path,'PATCH',{**fields,'expected_version':reassigned['version']},expected=404,key=key())
finally:grant('crm.activities.update','workspace')
grant('crm.activities.create','assigned')
try:call(base+'/crm/activities','POST',{**request,'type_id':interaction['id'],'responsible_membership_id':other},expected=403,key=key())
finally:grant('crm.activities.create','workspace')
# Archive retains history; ordinary edits reject until restored.
archived=call(path+'/archive','POST',{'expected_version':reassigned['version'],'archived':True},key=key())
call(path,'PATCH',{**fields,'expected_version':archived['version']},expected=409,key=key())
assert call(base+'/crm/activities'+query+'&archived=true')['items'][0]['id']==activity['id']
contact=call(base+'/contacts/'+contact['id']);contact=call(base+'/contacts/'+contact['id']+'/archive','POST',{'expected_version':contact['version'],'archived':True},key=key())
call(path+'/archive','POST',{'expected_version':archived['version'],'archived':False},expected=409,key=key())
assert len(call(path+'/history')['items'])>=5
call(base+'/crm/activities','POST',{**request,'type_id':interaction['id']},expected=409,key=key())
contact=call(base+'/contacts/'+contact['id']+'/archive','POST',{'expected_version':contact['version'],'archived':False},key=key())
restored=call(path+'/archive','POST',{'expected_version':archived['version'],'archived':False},key=key())
# Merged contacts retain original anchors and require both alias and survivor scope.
survivor=create('/contacts',{'display_name':name+' survivor'})
call(base+'/contacts/'+survivor['id']+'/merge','POST',{'source_id':contact['id'],'source_version':contact['version'],'survivor_version':survivor['version'],'resolved':{'display_name':survivor['display_name'],'notes':''}},key=key())
merged=call(path);assert merged['anchor_id']==contact['id'] and merged['canonical_anchor_id']==survivor['id']
assert {i['id'] for i in call(base+'/crm/activities?anchor_kind=contact&anchor_id='+survivor['id'])['items']}=={activity['id'],logged['id']}
f.sql("UPDATE contacts.contacts SET owner_membership_id='"+other+"' WHERE id='"+contact['id']+"'")
grant('contacts.read','assigned')
try:
 assert call(base+'/crm/activities?anchor_kind=contact&anchor_id='+survivor['id'])['items']==[]
 call(path,expected=404);call(path+'/history',expected=404)
finally:grant('contacts.read','workspace')
print('PASS: activity configuration, dates, history, rollback, responsibility/anchor scopes, archive and preserved merge references')
# Both CRM record anchors require their own permission and keep separate activity IDs.
pipeline=create('/crm/pipelines',{'name':name});stage=create('/crm/stages',{'name':name,'parent_id':pipeline['id'],'outcome':'open'})
category=create('/crm/categories',{'name':name});status=create('/crm/statuses',{'name':name,'parent_id':category['id']})
for anchor_kind,plural,body in [('opportunity','opportunities',{'pipeline_id':pipeline['id'],'stage_id':stage['id']}),('deal','deals',{'category_id':category['id'],'status_id':status['id']})]:
 record=create('/crm/'+plural,{'title':name,'owner_membership_id':other,**body})
 item=create('/crm/activities',{**request,'anchor_kind':anchor_kind,'anchor_id':record['id'],'type_id':interaction['id']})
 itempath=base+'/crm/activities/'+item['id'];q='?anchor_kind='+anchor_kind+'&anchor_id='+record['id']
 assert call(base+'/crm/activities'+q)['items']==[item]
 grant('crm.'+plural+'.read','assigned')
 try:call(itempath,expected=404);call(base+'/crm/activities'+q,expected=404)
 finally:grant('crm.'+plural+'.read','workspace')
 call(base+'/crm/activities'+q+'&after='+logged['id'],expected=404)
 call(base+'/crm/'+plural+'/'+record['id']+'/archive','POST',{'expected_version':record['version'],'archived':True},key=key())
 call(base+'/crm/activities','POST',{**request,'anchor_kind':anchor_kind,'anchor_id':record['id'],'type_id':interaction['id']},expected=409,key=key())
 assert call(itempath)==item
# Real workspace-qualified FKs reject foreign anchors and definition references.
foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('Activity foreign') RETURNING id").splitlines()[0]
foreign_member=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Foreign activity owner') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',id FROM u RETURNING id").splitlines()[0]
foreign_contact=f.sql("INSERT INTO contacts.contacts(workspace_id,display_name,owner_membership_id) VALUES('"+foreign+"','Foreign activity anchor','"+foreign_member+"') RETURNING id").splitlines()[0]
call(base+'/crm/activities','POST',{**request,'anchor_id':foreign_contact,'type_id':interaction['id']},expected=404,key=key())
f.sql("DO $$ BEGIN BEGIN UPDATE crm.activities SET contact_id='"+foreign_contact+"' WHERE id='"+logged['id']+"'; RAISE EXCEPTION 'Foreign anchor accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
f.sql("DO $$ BEGIN BEGIN UPDATE crm.activities SET deal_id='"+record['id']+"' WHERE id='"+logged['id']+"'; RAISE EXCEPTION 'Two anchors accepted'; EXCEPTION WHEN check_violation THEN NULL; END; END $$;")
# Authorized candidate pagination crosses hidden alias batches without leaking cursors.
f.sql("INSERT INTO crm.activities(workspace_id,contact_id,type_id,type_name,behavior,subject,occurred_at,responsible_membership_id) SELECT '"+f.workspace+"','"+contact['id']+"','"+interaction['id']+"','Interaction','interaction','Hidden alias',now(),'"+logged['responsible_membership_id']+"' FROM generate_series(1,105)")
visible=create('/crm/activities',{**request,'anchor_id':survivor['id'],'type_id':interaction['id']})
grant('contacts.read','assigned')
try:
 page=call(base+'/crm/activities?anchor_kind=contact&anchor_id='+survivor['id']+'&limit=1');assert [i['id'] for i in page['items']]==[visible['id']] and page['next_cursor'] is None
finally:grant('contacts.read','workspace')
# Concurrent versions and keys produce one event and one immutable revision per accepted write.
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request,urlopen
from urllib.error import HTTPError
import json
def concurrent_change(k,change):
 req=Request(f.base+base+'/crm/activities/'+visible['id'],method='PATCH',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':k},data=json.dumps(change).encode())
 try:r=urlopen(req,timeout=20)
 except HTTPError as e:r=e
 with r:return r.status,json.loads(r.read())
for same_key in [True,False]:
 current=call(base+'/crm/activities/'+visible['id']);payload={**fields,'type_id':interaction['id'],'expected_version':current['version']};k=key()
 with ThreadPoolExecutor(2) as executor:responses=list(executor.map(lambda pair:concurrent_change(*pair),[(k,payload),(k if same_key else key(),payload)]))
 assert sorted(r[0] for r in responses)==([200,200] if same_key else [200,409]),responses
 assert call(base+'/crm/activities/'+visible['id'])['version']==current['version']+1
assert len(call(base+'/crm/activities/'+visible['id']+'/history')['items'])==3
assert f.cli(['crm','activity-history',visible['id']])['items']
assert not f.mcp('crm_activity',{'workspace_id':f.workspace,'id':visible['id']}).get('isError')
module=next(m for m in call(base+'/modules') if m['key']=='crm');call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key())
try:
 call(path,expected=403);call(base+'/crm/activities'+query,expected=403);call(note_path,expected=403)
 call(base+'/crm/activities','POST',request,expected=403,key=retry)
 assert call(base+'/contacts/'+survivor['id'])
finally:
 module=next(m for m in call(base+'/modules') if m['key']=='crm');call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key())
print('PASS: opportunity/deal scopes, typed constraints, hidden batches, concurrent writes/retries, CLI/MCP and module independence')
# Server-rendered forms preserve invalid and stale drafts, keys and versions.
from browser_forms import Forms
from urllib.parse import urlencode,urlparse

def browser(path,values=None,expected=200):
 req=Request(f.base+path,data=None if values is None else urlencode(values).encode(),headers={'Origin':f.base,'Accept':'text/html'})
 try:response=f.browser.open(req,timeout=20)
 except HTTPError as failure:response=failure
 with response:
  html=response.read().decode();assert response.status==expected,(path,response.status,expected,html[:300]);return Forms(html),html,response.url
web='/workspaces/'+f.workspace+'/crm'
forms,_,_=browser(web+'/activity-types');draft=forms.form('name');draft.update(name=name+' web',behavior='unsupported')
forms,_,_=browser(web+'/activity-types',draft,400);assert forms.form('name')['behavior']=='unsupported'
draft['behavior']='interaction';forms,_,url=browser(web+'/activity-types',draft);type_id=urlparse(url).path.rsplit('/',1)[1]
edit_type=forms.form('name');edit_type['name']=name+' stale type'
current=call(base+'/crm/activity-types/'+type_id);call(base+'/crm/activity-types/'+type_id,'PATCH',{'name':name+' latest','behavior':'interaction','expected_version':current['version']},key=key())
forms,html,_=browser(web+'/activity-types/'+type_id,edit_type,409);assert forms.form('name')==edit_type and 'Discard draft' in html
webquery='?anchor_kind=contact&anchor_id='+survivor['id']
forms,_,_=browser(web+'/activities'+webquery);draft=forms.form('subject');draft.update(type_id=type_id,subject=name+' browser',occurred_at='2028-02-30T10:00:00Z')
forms,_,_=browser(web+'/activities'+webquery,draft,400);assert forms.form('subject')==draft
draft['occurred_at']='2028-02-29T10:00:00Z';forms,_,url=browser(web+'/activities'+webquery,draft);activity_id=urlparse(url).path.rsplit('/',1)[1]
assert call(base+'/crm/activities/'+activity_id)['type_id']==type_id
stale=forms.form('subject');stale.update(subject=name+' stale activity');current=call(base+'/crm/activities/'+activity_id)
call(base+'/crm/activities/'+activity_id,'PATCH',{**fields,'type_id':type_id,'subject':name+' server edit','expected_version':current['version']},key=key())
forms,html,_=browser(web+'/activities/'+activity_id+'?/save',stale,409);assert forms.form('subject')==stale and 'Discard draft' in html
forms,_,_=browser(web+'/activities/'+activity_id);archive=next(form['values'] for form in forms.forms if form['action']=='?/archive')
forms,_,_=browser(web+'/activities/'+activity_id+'?/archive',{**archive,'archived':'invalid'},400)
assert next(form['values'] for form in forms.forms if form['action']=='?/archive')['archived']=='invalid'
browser(web+'/activities/'+activity_id+'?/archive',archive)
assert call(base+'/crm/activities/'+activity_id)['archived']
print('PASS: browser type configuration, immutable behavior, activity recording, retained invalid/conflicting drafts and archive/history')
# Configured order governs cursor pages rather than only labelling definitions.
ordered_a=create('/crm/activity-types',{'name':name+' order first','behavior':'note','position':-2147483648})
ordered_b=create('/crm/activity-types',{'name':name+' order second','behavior':'note','position':-2147483647})
page=call(base+'/crm/activity-types?limit=1');assert page['items'][0]['id']==ordered_a['id'] and page['next_cursor']==ordered_a['id']
page=call(base+'/crm/activity-types?limit=1&after='+page['next_cursor']);assert page['items'][0]['id']==ordered_b['id']
call(base+'/crm/activity-types/'+ordered_a['id'],'PATCH',{'name':ordered_a['name'],'behavior':'note','position':2147483647,'expected_version':1},key=key())
assert call(base+'/crm/activity-types?limit=1')['items'][0]['id']==ordered_b['id']
# Retire the fixture's extreme positions so repeat executions remain independent.
for definition in [ordered_a,ordered_b]:
 current=call(base+'/crm/activity-types/'+definition['id']);call(base+'/crm/activity-types/'+definition['id'],'PATCH',{'name':current['name'],'behavior':'note','archived':True,'expected_version':current['version']},key=key())
# Type filter survives both pagination and archive navigation in SSR.
for _ in range(51):create('/crm/activities',{**request,'anchor_id':survivor['id'],'type_id':type_id})
_,html,_=browser(web+'/activities'+webquery+'&type_id='+type_id)
from html.parser import HTMLParser
class Links(HTMLParser):
 def __init__(self,html):super().__init__();self.urls=[];self.feed(html)
 def handle_starttag(self,tag,attrs):
  if tag=='a':self.urls.append(dict(attrs).get('href',''))
urls=Links(html).urls
assert any('after=' in u and 'type_id='+type_id in u for u in urls)
assert any('archived=true' in u and 'type_id='+type_id in u for u in urls)
print('PASS: configured order across cursor pages, retained SSR type filters and selected type navigation')

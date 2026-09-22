#!/usr/bin/env python3
"""CRM-owned labels respect record scopes and shared catalog lifecycle."""
import secrets
from integration_fixture import Fixture
f=Fixture();call=f.call;base='/api/v1/workspaces/'+f.workspace
name='CRM tags '+secrets.token_hex(5)
def key():return secrets.token_hex(16)
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if not module['enabled']:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
def create(path,body):return call(base+path,'POST',body,expected=201,key=key())
pipeline=create('/crm/pipelines',{'name':name});stage=create('/crm/stages',{'name':name,'parent_id':pipeline['id'],'outcome':'open'})
category=create('/crm/categories',{'name':name});status=create('/crm/statuses',{'name':name,'parent_id':category['id']})
tag=create('/tags',{'name':name});second=create('/tags',{'name':name+' second'})
for kind,body in [('opportunities',{'title':name,'pipeline_id':pipeline['id'],'stage_id':stage['id']}),('deals',{'title':name,'category_id':category['id'],'status_id':status['id']})]:
 record=create('/crm/'+kind,body);path=base+'/crm/'+kind+'/'+record['id']
 assert call(path+'/tags')==[]
 change={'expected_version':record['version'],'tag_ids':[tag['id']]};retry=key()
 assigned=call(path+'/tags','PUT',change,key=retry);assert assigned[0]['id']==tag['id']
 assert call(path+'/tags','PUT',change,key=retry)==assigned
 assert call(path)['version']==record['version']+1
 assert [r['id'] for r in call(base+'/crm/'+kind+'?tag_id='+tag['id'])['items']]==[record['id']]
 call(path+'/tags','PUT',{'expected_version':1,'tag_ids':[]},expected=409,key=key())
 print('PASS: '+kind+' label assignment, retry, parent version and filter')

 # Archive retains labels, permits removal, and denies new archived assignments.
 old=call(base+'/tags/'+tag['id'])
 call(base+'/tags/'+tag['id'],'PATCH',{'name':old['name'],'archived':True,'expected_version':old['version']},key=key())
 current=call(path)
 assert call(path+'/tags','PUT',{'expected_version':current['version'],'tag_ids':[tag['id']]},key=key())[0]['archived']
 call(path+'/archive','POST',{'expected_version':call(path)['version'],'archived':True},key=key())
 call(path+'/tags','PUT',{'expected_version':call(path)['version'],'tag_ids':[tag['id'],second['id']]},expected=409,key=key())
 call(path+'/tags','PUT',{'expected_version':call(path)['version'],'tag_ids':[]},key=key())
 assert call(path+'/tags')==[]
 call(path+'/archive','POST',{'expected_version':call(path)['version'],'archived':False},key=key())
 call(path+'/tags','PUT',{'expected_version':call(path)['version'],'tag_ids':[tag['id']]},expected=409,key=key())
 old=call(base+'/tags/'+tag['id']);call(base+'/tags/'+tag['id'],'PATCH',{'name':old['name'],'archived':False,'expected_version':old['version']},key=key())
 # Separate action scopes and foreign real IDs cannot be bypassed by tag assignment or filters.
 owner_role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
 other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('CRM tag scoped fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
 hidden=create('/crm/'+kind,{**body,'owner_membership_id':other})
 hidden_path=base+'/crm/'+kind+'/'+hidden['id']
 call(hidden_path+'/tags','PUT',{'expected_version':1,'tag_ids':[second['id']]},key=key())
 action='crm.'+kind+'.tags.manage'
 f.sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE role_id='"+owner_role+"' AND permission_key='"+action+"'")
 try:call(hidden_path+'/tags','PUT',{'expected_version':2,'tag_ids':[]},expected=404,key=key())
 finally:f.sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE role_id='"+owner_role+"' AND permission_key='"+action+"'")
 read='crm.'+kind+'.read'
 f.sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE role_id='"+owner_role+"' AND permission_key='"+read+"'")
 try:
  call(hidden_path+'/tags',expected=404)
  assert all(r['id']!=hidden['id'] for r in call(base+'/crm/'+kind+'?tag_id='+second['id']+'&limit=1')['items'])
 finally:f.sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE role_id='"+owner_role+"' AND permission_key='"+read+"'")
 foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('CRM tag foreign fixture') RETURNING id").splitlines()[0]
 foreign_tag=f.sql("INSERT INTO suite.tags(workspace_id,name,name_key) VALUES('"+foreign+"','Foreign','foreign') RETURNING id").splitlines()[0]
 call(path+'/tags','PUT',{'expected_version':call(path)['version'],'tag_ids':[foreign_tag]},expected=404,key=key())
 call(base+'/crm/'+kind+'?tag_id='+foreign_tag,expected=404)
 # Failed audit and stale replay authority leave relationships and parent version unchanged.
 version=call(path)['version'];f.sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
 try:call(path+'/tags','PUT',{'expected_version':version,'tag_ids':[second['id']]},expected=503,key=key())
 finally:f.sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
 assert call(path)['version']==version and call(path+'/tags')==[]
 module=next(m for m in call(base+'/modules') if m['key']=='crm')
 module=call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':False},key=key())
 try:
  call(path+'/tags',expected=403)
  call(path+'/tags','PUT',change,expected=403,key=retry)
 finally:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
 singular='opportunity' if kind=='opportunities' else 'deal'
 assert f.cli(['crm',singular+'-tags',record['id']])==[]
 assert not f.mcp('crm_set_'+singular+'_tags',{'workspace_id':f.workspace,'id':record['id'],'change':{'expected_version':version,'tag_ids':[second['id']]},'idempotency_key':key()}).get('isError')
 print('PASS: '+kind+' tag archive/restore, per-action scope, foreign IDs, audit rollback, module disable and CLI/MCP')

 # Server-rendered tags use the same operation and preserve rejected drafts.
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
 web='/workspaces/'+f.workspace+'/crm/'+kind+'/'+record['id']
 forms,_=browser(web)
 form=next(form['values'] for form in forms.forms if form['action']=='?/tags')
 stale={**form,'tag_id':tag['id']}
 call(path+'/tags','PUT',{'expected_version':call(path)['version'],'tag_ids':[second['id']]},key=key())
 forms,html=browser(web+'?/tags',stale,409)
 retained=next(form['values'] for form in forms.forms if form['action']=='?/tags')
 for field in ['idempotency_key','expected_version','tag_id']:assert retained[field]==stale[field],(field,retained,stale)
 assert 'Discard tag draft and reload saved values' in html
 forms,_=browser(web);form=next(form['values'] for form in forms.forms if form['action']=='?/tags')
 browser(web+'?/tags',{**form,'tag_id':tag['id']})
 assert [t['id'] for t in call(path+'/tags')]==[tag['id']]
 _,html=browser('/workspaces/'+f.workspace+'/tags?search='+name.replace(' ','%20'))
 assert '/crm/'+kind+'?tag_id='+tag['id'] in html
 _,html=browser('/workspaces/'+f.workspace+'/crm/'+kind+'?tag_id='+tag['id'])
 assert '/crm/'+kind+'/'+record['id'] in html
 print('PASS: '+kind+' browser tagging, retained conflicting draft and full-catalog filters')
 # Two simultaneous replacement sets cannot both overwrite the same version.
 from concurrent.futures import ThreadPoolExecutor
 import json
 from urllib.request import urlopen
 expected_version=call(path)['version']
 def race(ids):
  request=Request(f.base+path+'/tags',method='PUT',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':key()},data=json.dumps({'expected_version':expected_version,'tag_ids':ids}).encode())
  try:response=urlopen(request,timeout=20)
  except HTTPError as failure:response=failure
  with response:return response.status
 with ThreadPoolExecutor(max_workers=2) as pool:assert sorted(pool.map(race,[[tag['id']],[second['id']]]))==[200,409]
 call(path+'/tags','PUT',{'expected_version':call(path)['version'],'tag_ids':[tag['id'],tag['id']]},expected=400,key=key())
 f.sql("DO $$ BEGIN BEGIN INSERT INTO crm."+singular+"_tags(workspace_id,"+singular+"_id,tag_id,created_by_membership_id) VALUES('"+f.workspace+"','"+record['id']+"','"+foreign_tag+"','"+other+"'); RAISE EXCEPTION 'Foreign tag accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
 print('PASS: '+kind+' concurrent tag edits, duplicate validation and database workspace FK')

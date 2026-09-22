#!/usr/bin/env python3
"""CRM participants preserve independent reference visibility and association history."""
import secrets
from integration_fixture import Fixture
f=Fixture();call=f.call;base='/api/v1/workspaces/'+f.workspace
name='CRM participants '+secrets.token_hex(5)
def key():return secrets.token_hex(16)
def create(path,body):return call(base+path,'POST',body,expected=201,key=key())
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if not module['enabled']:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
pipeline=create('/crm/pipelines',{'name':name});stage=create('/crm/stages',{'name':name,'parent_id':pipeline['id'],'outcome':'open'})
opportunity=create('/crm/opportunities',{'title':name,'pipeline_id':pipeline['id'],'stage_id':stage['id']})
contact=create('/contacts',{'display_name':name})
path=base+'/crm/opportunities/'+opportunity['id']
assert call(path+'/contacts')['items']==[]
participant=call(path+'/contacts','POST',{'expected_version':1,'reference_id':contact['id'],'role_label':'Buyer','is_primary':True},expected=201,key=key())
assert participant['canonical_id']==contact['id'] and participant['is_primary']
assert len(call(path+'/contacts/'+participant['id']+'/history')['items'])==1
print('PASS: opportunity contact assignment, independent identity and immutable association history')
# Exercise all four typed associations with independent scope and lifecycle checks.
category=create('/crm/categories',{'name':name});status=create('/crm/statuses',{'name':name,'parent_id':category['id']})
owner_role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Participant scoped fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
def scope(action,value):f.sql("UPDATE suite.role_permissions SET record_scope='"+value+"' WHERE role_id='"+owner_role+"' AND permission_key='"+action+"'")
for sales,singular,record_body in [('opportunities','opportunity',{'title':name,'pipeline_id':pipeline['id'],'stage_id':stage['id']}),('deals','deal',{'title':name,'category_id':category['id'],'status_id':status['id']})]:
 for refs,reference,display in [('contacts','contact','display_name'),('organizations','organization','name')]:
  record=create('/crm/'+sales,record_body);path=base+'/crm/'+sales+'/'+record['id'];collection=path+'/'+refs
  def ref(label,**values):return create('/'+refs,{display:name+' '+label,**values})
  def version():return call(path)['version']
  def add(ref_id,**values):return call(collection,'POST',{'expected_version':version(),'reference_id':ref_id,**values},expected=201,key=key())
  def edit(p,**values):return call(collection+'/'+p['id'],'PATCH',{'expected_version':version(),**values},key=key())
  first=ref('first');second=ref('second');primary_body={'expected_version':version(),'reference_id':first['id'],'role_label':'Buyer','is_primary':True};primary_key=key()
  first_p=call(collection,'POST',primary_body,expected=201,key=primary_key)
  assert call(collection,'POST',primary_body,expected=201,key=primary_key)==first_p
  second_p=add(second['id'],role_label='Adviser')
  second_p=edit(second_p,is_primary=True)
  first_history=call(collection+'/'+first_p['id']+'/history')
  assert not first_history['participant']['is_primary'] and len(first_history['items'])==2
  second_p=edit(second_p,role_label='Lead adviser');assert second_p['is_primary']
  action='contacts.read' if refs=='contacts' else 'contacts.organizations.read'
  # Primary promotion uses a uniform broad-visibility prerequisite; demotion/end does not.
  scope(action,'assigned')
  try:
   call(collection,'POST',primary_body,expected=403,key=primary_key)
   call(collection+'/'+first_p['id'],'PATCH',{'expected_version':version(),'is_primary':True},expected=403,key=key())
   second_p=edit(second_p,ended=True);assert second_p['ended'] and not second_p['is_primary']
  finally:scope(action,'workspace')
  second_p=edit(second_p,ended=False,is_primary=True)
  # Archive is reversible and does not destroy participant history; restore is a new assignment.
  call(base+'/'+refs+'/'+second['id']+'/archive','POST',{'expected_version':second['version'],'archived':True},key=key())
  assert call(collection+'/'+second_p['id']+'/history')['participant']['reference_archived']
  second_p=edit(second_p,ended=True)
  call(collection+'/'+second_p['id'],'PATCH',{'expected_version':version(),'ended':False},expected=409,key=key())
  second_current=call(base+'/'+refs+'/'+second['id'])
  call(base+'/'+refs+'/'+second['id']+'/archive','POST',{'expected_version':second_current['version'],'archived':False},key=key())
  second_p=edit(second_p,ended=False)
  # Hidden candidates span more than a database batch; visible cursor pages remain complete.
  f.sql("WITH refs AS (INSERT INTO contacts."+refs+"(workspace_id,"+display+",owner_membership_id) SELECT '"+f.workspace+"','Hidden batch '||n,'"+other+"' FROM generate_series(1,105) n RETURNING id) INSERT INTO crm."+singular+'_'+refs+"(workspace_id,"+singular+'_id,'+reference+"_id,created_by_membership_id,created_at) SELECT '"+f.workspace+"','"+record['id']+"',id,'"+other+"',now()-interval '1 day' FROM refs")
  hidden_id=f.sql('SELECT id FROM crm.'+singular+'_'+refs+" WHERE workspace_id='"+f.workspace+"' AND "+singular+"_id='"+record['id']+"' ORDER BY created_at,id LIMIT 1")
  scope(action,'assigned')
  try:
   page=call(collection+'?limit=1');assert len(page['items'])==1 and page['next_cursor']
   page2=call(collection+'?limit=1&after='+page['next_cursor']);assert len(page2['items'])==1 and page2['next_cursor'] is None
   assert {page['items'][0]['id'],page2['items'][0]['id']}=={first_p['id'],second_p['id']}
   call(collection+'?after='+hidden_id,expected=404)
   call(collection+'/'+hidden_id+'/history',expected=404)
   call(collection+'/'+hidden_id,'PATCH',{'expected_version':version(),'ended':True},expected=404,key=key())
  finally:scope(action,'workspace')
  # Parent write scope does not follow broad read; links do not grant reference access.
  hidden_record=create('/crm/'+sales,{**record_body,'owner_membership_id':other})
  scope('crm.'+sales+'.participants.manage','assigned')
  try:call(base+'/crm/'+sales+'/'+hidden_record['id']+'/'+refs,'POST',{'expected_version':1,'reference_id':first['id']},expected=404,key=key())
  finally:scope('crm.'+sales+'.participants.manage','workspace')
  call(base+'/crm/'+sales+'/'+hidden_record['id']+'/'+refs+'/'+first_p['id']+'/history',expected=404)
  # Audit failure rolls association, history and parent version back together.
  third=ref('third');prior=version();counts=f.sql('SELECT count(*) FROM crm.'+singular+'_'+refs)
  f.sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
  try:call(collection,'POST',{'expected_version':prior,'reference_id':third['id']},expected=503,key=key())
  finally:f.sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
  assert version()==prior and f.sql('SELECT count(*) FROM crm.'+singular+'_'+refs)==counts
  # API, CLI and MCP adapters share these business operations.
  assert f.cli(['crm',singular+'-'+refs,record['id']])['items']
  assert not f.mcp('crm_edit_'+singular+'_'+reference,{'workspace_id':f.workspace,'id':record['id'],'association_id':first_p['id'],'change':{'expected_version':version(),'role_label':'CLI/MCP checked'},'idempotency_key':key()}).get('isError')
  module=next(m for m in call(base+'/modules') if m['key']=='crm');module=call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':False},key=key())
  try:
   call(collection,expected=403)
   call(collection,'POST',primary_body,expected=403,key=primary_key)
  finally:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
  print('PASS: '+singular+' '+reference+' primary history, scoped demotion/promotion/replay, archive/restore, hidden pagination, parent scope, rollback and CLI/MCP')
  if refs=='contacts':
   # A hidden source alias must not make duplicate detection an existence oracle.
   source=ref('hidden merge source',owner_membership_id=other);survivor=ref('visible survivor')
   original=add(source['id'],role_label='Historical role')
   call(base+'/contacts/'+survivor['id']+'/merge','POST',{'source_id':source['id'],'source_version':source['version'],'survivor_version':survivor['version'],'resolved':{'display_name':survivor['display_name']}},key=key())
   assert call(collection+'/'+original['id']+'/history')['participant']['canonical_id']==survivor['id']
   scope(action,'assigned')
   try:
    call(collection+'/'+original['id']+'/history',expected=404)
    new=add(survivor['id'],role_label='Current role')
    assert new['canonical_id']==survivor['id'] and new['id']!=original['id']
   finally:scope(action,'workspace')
   assert call(collection+'/'+original['id']+'/history')['participant']['role_label']=='Historical role'
   call(collection,'POST',{'expected_version':version(),'reference_id':survivor['id']},expected=409,key=key())
   print('PASS: alias/survivor scope, preserved merge associations and non-disclosing canonical duplicate checks')
  from browser_forms import Forms
  from urllib.request import Request
  from urllib.parse import urlencode
  from urllib.error import HTTPError
  def browser(path,body=None,expected=200):
   request=Request(f.base+path,data=None if body is None else urlencode(body).encode(),headers={'Origin':f.base,'Accept':'text/html'})
   try:response=f.browser.open(request,timeout=20)
   except HTTPError as failure:response=failure
   with response:
    html=response.read().decode();assert response.status==expected,(path,response.status,html[:300]);return response.geturl(),Forms(html),html
  web='/workspaces/'+f.workspace+'/crm/'+sales+'/'+record['id']+'/participants/'+refs
  _,forms,_=browser(web+'?search=NoMatchingFixture')
  draft={**forms.form('idempotency_key'),'reference_id':third['id'],'role_label':'x'*81,'primary':'no'}
  _,forms,_=browser(web+'?search=NoMatchingFixture',draft,400)
  retained=forms.form('idempotency_key')
  assert retained['reference_id']==third['id'] and retained['idempotency_key']==draft['idempotency_key']
  url,forms,_=browser(web,{**retained,'role_label':'Browser added'})
  browser_id=url.rsplit('/',1)[1];detail=web+'/'+browser_id
  form=forms.form('idempotency_key');before=call(collection+'/'+browser_id+'/history');prior=version()
  for bad in [{'primary':'invalid'},{'state':'invalid'}]:browser(detail,{**form,**bad},400)
  assert version()==prior and call(collection+'/'+browser_id+'/history')==before
  edit({'id':browser_id},role_label='Concurrent edit')
  _,forms,html=browser(detail,{**form,'role_label':'Retained draft'},409)
  retained=forms.form('idempotency_key');assert retained['role_label']=='Retained draft' and retained['expected_version']==form['expected_version'] and retained['idempotency_key']==form['idempotency_key']
  assert 'Discard draft and reload saved values' in html
  _,forms,_=browser(detail);form=forms.form('idempotency_key')
  browser(detail,{**form,'role_label':'Browser edit','primary':'keep','state':'keep'})
  assert call(collection+'/'+browser_id+'/history')['participant']['role_label']=='Browser edit'
  # Forced overlap: archive commits while the linking operation waits on its row lock.
  import subprocess
  def held_transaction(statement,operation):
   process=subprocess.Popen(['docker','compose','exec','-T','db','psql','-U','postgres','-d','postgres','-At','-v','ON_ERROR_STOP=1'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1,env=f.environment)
   audit="INSERT INTO suite.audit_events(workspace_id,operation_id,actor_user_id,action,source_interface) SELECT workspace_id,gen_random_uuid(),user_id,'fixture.reference_overlap','operator' FROM suite.memberships WHERE id='"+other+"'"
   process.stdin.write("BEGIN;\n"+statement+";\nSELECT 'LOCKED';\nSELECT pg_sleep(1);\n"+audit+";\nCOMMIT;\n");process.stdin.close()
   while True:
    line=process.stdout.readline();assert line,'Fixture lock was not acquired'
    if line.strip()=='LOCKED':break
   try:operation()
   finally:
    process.wait(timeout=10);assert process.returncode==0,process.stderr.read();process.stdout.close();process.stderr.close()
  archived_ref=ref('concurrent archive')
  held_transaction("UPDATE contacts."+refs+" SET archived_at=now(),version=version+1 WHERE workspace_id='"+f.workspace+"' AND id='"+archived_ref['id']+"'",lambda:call(collection,'POST',{'expected_version':version(),'reference_id':archived_ref['id']},expected=409,key=key()))
  if refs=='contacts':
   merge_source=ref('concurrent merge source');merge_target=ref('concurrent merge survivor')
   result=[]
   held_transaction("SELECT pg_advisory_xact_lock(hashtextextended('contact-merge:"+f.workspace+"',0)); UPDATE contacts.contacts SET merged_into_id='"+merge_target['id']+"',archived_at=now(),version=version+1 WHERE workspace_id='"+f.workspace+"' AND id='"+merge_source['id']+"'",lambda:result.append(add(merge_source['id'])))
   assert result[0]['reference_id']==merge_target['id'] and result[0]['canonical_id']==merge_target['id']
  print('PASS: '+singular+' '+reference+' web drafts, malformed forms, stale edits and forced reference archive/merge overlap')
  # Reassignment cannot race an independently scoped reference check.
  reassigned=ref('concurrent reassignment')
  scope(action,'assigned')
  try:
   held_transaction("UPDATE contacts."+refs+" SET owner_membership_id='"+other+"',version=version+1 WHERE workspace_id='"+f.workspace+"' AND id='"+reassigned['id']+"'",lambda:call(collection,'POST',{'expected_version':version(),'reference_id':reassigned['id']},expected=404,key=key()))
  finally:scope(action,'workspace')
  # Same-version primary promotions serialize; the loser leaves no extra history.
  from concurrent.futures import ThreadPoolExecutor
  from urllib.request import urlopen
  import json
  expected=version()
  def promote(association):
   request=Request(f.base+collection+'/'+association,method='PATCH',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':key()},data=json.dumps({'expected_version':expected,'is_primary':True}).encode())
   try:response=urlopen(request,timeout=20)
   except HTTPError as failure:response=failure
   with response:return response.status
  with ThreadPoolExecutor(max_workers=2) as pool:assert sorted(pool.map(promote,[first_p['id'],second_p['id']]))==[200,409]
  assert f.sql('SELECT count(*) FROM crm.'+singular+'_'+refs+" WHERE workspace_id='"+f.workspace+"' AND "+singular+"_id='"+record['id']+"' AND is_primary")=='1'
  foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('Participant foreign fixture') RETURNING id").splitlines()[0]
  foreign_member=f.sql("INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',user_id FROM suite.memberships WHERE id='"+other+"' RETURNING id").splitlines()[0]
  foreign_ref=f.sql('INSERT INTO contacts.'+refs+'(workspace_id,'+display+",owner_membership_id) VALUES('"+foreign+"','Foreign','"+foreign_member+"') RETURNING id").splitlines()[0]
  call(collection,'POST',{'expected_version':version(),'reference_id':foreign_ref},expected=404,key=key())
  f.sql('DO $$ BEGIN BEGIN INSERT INTO crm.'+singular+'_'+refs+'(workspace_id,'+singular+'_id,'+reference+"_id,created_by_membership_id) VALUES('"+f.workspace+"','"+record['id']+"','"+foreign_ref+"','"+other+"'); RAISE EXCEPTION 'Foreign participant accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
  print('PASS: '+singular+' '+reference+' concurrent ownership/primary changes and real foreign-workspace references')

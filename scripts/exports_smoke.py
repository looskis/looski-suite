#!/usr/bin/env python3
"""Export uses public business projections and independent current authority."""
import json,secrets
from urllib.parse import urlencode
from urllib.request import Request
from urllib.error import HTTPError
from integration_fixture import Fixture
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}'
def key():return secrets.token_hex(16)
def create(path,body):return f.call(base+path,'POST',body,expected=201,key=key())
def page(collection,expected=200,**selection):return f.call(base+'/exports/page','POST',{'collection':collection,**selection},expected=expected)
def all_items(collection,**selection):
 result=page(collection,**selection);items=result['items'];seen=set()
 while result['next']:
  cursor=result['next']['after'];assert cursor not in seen;seen.add(cursor)
  result=f.call(base+'/exports/page','POST',result['next']);items+=result['items']
 return items
contact=create('/contacts',{'display_name':'=SUM(A1:A2) '+key(),'notes':'Faithful\n+not a formula\n雪','identifiers':[{'kind':'email','value':'Case'+key()+'@Example.INVALID'}]})
organization=create('/organizations',{'name':'Export organization '+key()})
task=create('/tasks',{'title':'Export task '+key(),'status':'open'})
pipeline=create('/crm/pipelines',{'name':'Export pipeline '+key()})
stage=create('/crm/stages',{'name':'Export stage','parent_id':pipeline['id'],'outcome':'open'})
category=create('/crm/categories',{'name':'Export category '+key()})
status=create('/crm/statuses',{'name':'Export status','parent_id':category['id']})
opportunity=create('/crm/opportunities',{'title':'Export potential','pipeline_id':pipeline['id'],'stage_id':stage['id'],'expected_value':'99999999999999.999999','currency_code':'USD'})
deal=create('/crm/deals',{'title':'Export actual','category_id':category['id'],'status_id':status['id'],'originating_opportunity_id':opportunity['id']})
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")

def grant(action,scope):f.sql("UPDATE suite.role_permissions SET record_scope='"+scope+"' WHERE role_id='"+role+"' AND permission_key='"+action+"'")
# The new grant is necessary, and is not a replacement for existing read grants.
f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='workspace.data.export'")
try:
 page('contacts',403);f.call(base+'/exports/collections',expected=403)
 assert f.call(base+'/contacts/'+contact['id'])==contact
 result=f.mcp('export_page',{'workspace_id':f.workspace,'selection':{'collection':'contacts'}});assert result['isError']
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','workspace.data.export')")
print('PASS: explicit export permission is required across transports; ordinary reads remain independent')

catalog=f.call(base+'/exports/collections')
parents={'contact_organizations':contact['id'],'contact_tags':contact['id'],'organization_tags':organization['id'],'task_history':task['id'],'contact_activities':contact['id']}
for name in ['opportunity_contacts','opportunity_organizations','opportunity_tasks','opportunity_history','opportunity_tags','opportunity_activities']:parents[name]=opportunity['id']
for name in ['deal_contacts','deal_organizations','deal_tasks','deal_history','deal_tags','deal_activities','deal_coordination']:parents[name]=deal['id']
for name,table in [('task_revisions','tasks.templates'),('coordination_revisions','crm.coordination_templates'),('activity_history','crm.activities')]:
 parents[name]=f.sql("SELECT id FROM "+table+" WHERE workspace_id='"+f.workspace+"' ORDER BY id LIMIT 1")
 assert parents[name],name+' requires the earlier capability fixtures'
for spec in catalog:
 selection={'limit':1}
 if spec['parent_required']:selection['parent_id']=parents[spec['collection']]
 result=page(spec['collection'],**selection)
 assert result['manifest']['format']=='looski-business-json' and result['manifest']['format_version']==1
 assert result['manifest']['workspace_id']==f.workspace and len(result['items'])<=1
 if result['next']:
  next_page=f.call(base+'/exports/page','POST',result['next']);assert len(next_page['items'])<=1
  assert next_page['items']!=result['items']
print('PASS: all documented collections return bounded public-contract pages with stable manifests and usable continuations')

exported=next(c for c in all_items('contacts') if c['id']==contact['id']);assert exported==contact
exported=next(o for o in all_items('opportunities') if o['id']==opportunity['id']);assert exported==opportunity
exported=next(d for d in all_items('deals') if d['id']==deal['id']);assert exported==deal
assert next(s for s in all_items('stages') if s['id']==stage['id'])==stage
print('PASS: exact strings, case-sensitive identifiers, decimal values, origins and custom definitions survive JSON export')

other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Export hidden member') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
hidden=create('/contacts',{'display_name':'Hidden export contact','owner_membership_id':other})
hidden_org=create('/organizations',{'name':'Hidden export organization','owner_membership_id':other})
f.call(base+'/contacts/'+contact['id']+'/organizations/'+hidden_org['id'],'PUT',{'expected_contact_version':contact['version'],'relationship_label':'Hidden'},key=key())
relation=page('contact_organizations',parent_id=contact['id']);assert len(relation['items'])==1
for action in ['contacts.read','contacts.organizations.read']:
 grant(action,'assigned')
 try:
  if action=='contacts.read':
   assert hidden['id'] not in [r['id'] for r in all_items('contacts')]
   page('contact_tags',404,parent_id=hidden['id'])
  else:assert page('contact_organizations',parent_id=contact['id'])['items']==[]
 finally:grant(action,'workspace')
current=f.call(base+'/contacts/'+contact['id'])
f.call(base+'/contacts/'+contact['id']+'/organizations/'+hidden_org['id'],'PUT',{'expected_contact_version':current['version'],'ended':True},key=key())
assert page('contact_organizations',parent_id=contact['id'])['items']==[]
assert page('contact_organizations',parent_id=contact['id'],ended=True)['items'][0]['ended']
print('PASS: assigned visibility, hidden parents and independently scoped current/historical relationships')

# Tag sets larger than a page retain every stable assignment.
tag_ids=[create('/tags',{'name':'Export tag '+key()})['id'] for _ in range(26)]
current=f.call(base+'/contacts/'+contact['id'])
f.call(base+'/contacts/'+contact['id']+'/tags','PUT',{'expected_version':current['version'],'tag_ids':tag_ids},key=key())
assert set(r['id'] for r in all_items('contact_tags',parent_id=contact['id']))==set(tag_ids)
archived=f.call(base+'/contacts/'+contact['id']+'/archive','POST',{'expected_version':f.call(base+'/contacts/'+contact['id'])['version'],'archived':True},key=key())
assert contact['id'] not in [r['id'] for r in all_items('contacts')]
assert next(r for r in all_items('contacts',archived=True) if r['id']==contact['id'])==archived
print('PASS: paginated tag relationships and explicit archive selection retain complete selected data')

first=page('contacts',limit=1);assert first['next']
f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='workspace.data.export'")
try:f.call(base+'/exports/page','POST',first['next'],expected=403)
finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','workspace.data.export')")
module=next(m for m in f.call(base+'/modules') if m['key']=='crm')
module=f.call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key())
try:
 page('opportunities',403);page('deals',403);page('contact_activities',403,parent_id=contact['id']);page('contacts')
finally:f.call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key())
for selection in [{'limit':0},{'limit':26},{'parent_id':contact['id']},{'ended':True}]:page('contacts',400,**selection)
page('contact_tags',400)
f.call('/api/v1/workspaces/00000000-0000-0000-0000-000000000000/exports/page','POST',{'collection':'contacts'},expected=403)
f.call(base+'/exports/page','POST',{'collection':'contacts'},expected=401,authenticated=False)
print('PASS: revoked export authority between pages, foreign workspace and module denial, strict bounded selections')

selection={'collection':'contacts','limit':1}
expected=page(**selection)
assert f.cli(['export','page'],selection)==expected
assert f.cli(['export','collections'])==catalog
result=f.mcp('export_page',{'workspace_id':f.workspace,'selection':selection});assert not result.get('isError')
assert json.loads(result['content'][0]['text'])==expected
query=urlencode({'collection':'contacts','run':'true'})
with f.browser.open(f.base+f'/workspaces/{f.workspace}/exports?'+query) as response:
 html=response.read().decode();assert response.status==200 and 'Download this page' in html
with f.browser.open(f.base+f'/workspaces/{f.workspace}/exports/download?collection=contacts') as response:
 assert response.headers['Content-Disposition']=='attachment; filename="looski-export-page.json"'
 assert response.headers['Cache-Control']=='no-store'
 assert json.loads(response.read())==page('contacts',limit=25,archived=False)
print('PASS: API/CLI/MCP export parity and private browser preview/download with continuation')

# The agent-friendly collector finalizes only after every continuation succeeds.
from export_collection import collect
from pathlib import Path
import tempfile
with tempfile.TemporaryDirectory(dir=f.directory) as directory:
 output=Path(directory)/'contacts.json'
 result=collect(lambda selection:f.cli(['export','page'],selection),{'collection':'contact_tags','parent_id':contact['id'],'limit':5},output,workspace=f.workspace)
 package=json.loads(output.read_text());assert package['complete'] and result['pages']==6 and result['records']==26
 assert output.stat().st_mode&0o777==0o600
 assert set(row['id'] for p in package['pages'] for row in p['items'])==set(tag_ids)
 failed=Path(directory)/'failed.json';attempts=[]
 def revoked(selection):
  if attempts:
   f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='workspace.data.export'")
  attempts.append(selection)
  return f.call(base+'/exports/page','POST',selection)
 try:
  try:collect(revoked,{'collection':'contact_tags','parent_id':contact['id'],'limit':1},failed,workspace=f.workspace)
  except AssertionError:pass
  else:raise AssertionError('Revoked export unexpectedly completed')
 finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','workspace.data.export') ON CONFLICT DO NOTHING")
 assert not failed.exists() and list(Path(directory).iterdir())==[output]
print('PASS: all-page private artifact, continuation fidelity and cleanup on real mid-export revocation')

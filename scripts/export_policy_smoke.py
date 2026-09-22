#!/usr/bin/env python3
"""Focused export policy regressions: missing grants, merges and scoped configuration."""
import secrets
from integration_fixture import Fixture
f=Fixture();base=f'/api/v1/workspaces/{f.workspace}'
def key():return secrets.token_hex(16)
def create(path,body):return f.call(base+path,'POST',body,expected=201,key=key())
def export(collection,**query):return f.call(base+'/exports/page','POST',{'collection':collection,**query})
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
deal=f.call(base+'/crm/deals')['items'][0]
failures=[]
for action,collection in [('tasks.read','deal_tasks'),('contacts.read','deal_contacts'),('contacts.organizations.read','deal_organizations')]:
 f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='"+action+"'")
 try:
  try:f.call(base+'/exports/page','POST',{'collection':collection,'parent_id':deal['id']},expected=403)
  except AssertionError:failures.append(collection+' falsely completed without '+action)
 finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','"+action+"')")
source=create('/contacts',{'display_name':'Export source '+key()});survivor=create('/contacts',{'display_name':'Export survivor '+key()})
organization=create('/organizations',{'name':'Export retained '+key()})
f.call(base+'/contacts/'+source['id']+'/organizations/'+organization['id'],'PUT',{'expected_contact_version':source['version'],'relationship_label':'Original relationship'},key=key())
original=export('contact_organizations',parent_id=source['id'])['items'][0]
source=f.call(base+'/contacts/'+source['id'])
f.call(base+'/contacts/'+survivor['id']+'/merge','POST',{'source_id':source['id'],'source_version':source['version'],'survivor_version':survivor['version'],'resolved':{'display_name':survivor['display_name']}},key=key())
for parent in [source['id'],survivor['id']]:
 history=export('contact_organizations',parent_id=parent,ended=True)['items']
 if not any(row['id']==original['id'] and row['contact_id']==source['id'] and row['ended'] for row in history):failures.append('Merged original history missing from '+parent)
assert not failures,failures
print('PASS: absent dependent grants fail explicitly and authorized merged-contact relationship history retains original IDs')

# Hidden merged aliases never become visible through the canonical contact's history.
other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Export alias scope') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
f.sql("UPDATE contacts.contacts SET owner_membership_id='"+other+"' WHERE id='"+source['id']+"'; UPDATE suite.role_permissions SET record_scope='assigned' WHERE role_id='"+role+"' AND permission_key='contacts.read'")
try:
 history=export('contact_organizations',parent_id=survivor['id'],ended=True)['items']
 assert all(row['contact_id']!=source['id'] for row in history)
 f.call(base+'/exports/page','POST',{'collection':'contact_organizations','parent_id':source['id'],'ended':True},expected=404)
finally:f.sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE role_id='"+role+"' AND permission_key='contacts.read'")
print('PASS: merged relationship history intersects original and survivor visibility')

# Coordination exports include the instantiated checklist, not just its heading.
parent=f.sql("SELECT deal_id FROM crm.deal_coordination_instances WHERE workspace_id='"+f.workspace+"' ORDER BY id LIMIT 1");assert parent
instances=export('deal_coordination',parent_id=parent)
assert instances['items']
for instance in instances['items']:
 assert instance==f.call(base+'/crm/deals/'+parent+'/coordination/'+instance['id'])
 assert 'items' in instance
rules=export('coordination_rules')
assert rules['items']==f.call(base+'/crm/coordination-rules?limit=25')['items']
print('PASS: coordination item/link/task identities and rule configuration are exportable')

from browser_forms import Forms
from urllib.parse import urlencode
for selection in [{'collection':'contact_organizations','parent_id':survivor['id'],'ended':'true'}, {'collection':'contacts','archived':'true'}]:
 with f.browser.open(f.base+f'/workspaces/{f.workspace}/exports?'+urlencode({'run':'true',**selection})) as response:
  form=Forms(response.read().decode()).form('collection')
  for field,value in selection.items():assert form[field]==value,(field,form[field],value)
print('PASS: browser retains collection, parent and lifecycle selection across server-rendered navigation')

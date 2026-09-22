#!/usr/bin/env python3
"""Organization and temporal relationship acceptance on a dedicated fixture."""
import secrets
from concurrent.futures import ThreadPoolExecutor
from integration_fixture import Fixture

fixture=Fixture()
call,sql=fixture.call,fixture.sql
prefix='/api/v1/workspaces/'+fixture.workspace
orgs=prefix+'/organizations'
contacts=prefix+'/contacts'
name='Organizations '+secrets.token_hex(6)
def key():return secrets.token_hex(16)

def create_contact(label):return call(contacts,'POST',{'display_name':name+' '+label},expected=201,key=key())
def get_contact(contact):return call(contacts+'/'+contact['id'])
def relate(contact,org,**fields):
    return call(contacts+'/'+contact['id']+'/organizations/'+org['id'],'PUT',{'expected_contact_version':get_contact(contact)['version'],**fields},key=key())
def relationships(contact):return call(contacts+'/'+contact['id']+'/organizations')

call(orgs,expected=401,authenticated=False)
body={'name':name,'website_url':'https://example.com','notes':'Original'}
creation_key=key()
organization=call(orgs,'POST',body,expected=201,key=creation_key)
assert call(orgs,'POST',body,expected=201,key=creation_key)==organization
call(orgs,'POST',{**body,'name':'Changed'},expected=409,key=creation_key)
same_name=call(orgs,'POST',body,expected=201,key=key())
assert same_name['id']!=organization['id'], 'Names/domains are not organization identity'
call(orgs,'POST',{'name':'Invalid','website_url':'javascript:alert(1)'},expected=400,key=key())
edited=call(orgs+'/'+organization['id'],'PATCH',{'name':name+' edited','website_url':'https://example.org','expected_version':1},key=key())
assert edited['version']==2
call(orgs+'/'+organization['id'],'PATCH',{'name':'Stale','expected_version':1},expected=409,key=key())
assert any(o['id']==organization['id'] for o in call(orgs+'?search='+name.replace(' ','%20')+'&limit=100')['items'])

person=create_contact('person')
first=relate(person,organization,relationship_label='Customer',job_title='Director',is_primary=True)
second=relate(person,same_name,relationship_label='Advisor',is_primary=True)
current=relationships(person)
assert len(current)==2 and sum(r['is_primary'] for r in current)==1
assert next(r for r in current if r['organization_id']==same_name['id'])['is_primary']
assert sql("SELECT ended_at IS NOT NULL FROM contacts.contact_organizations WHERE id='"+first['id']+"'")=='t'
call(contacts+'/'+person['id']+'/organizations/'+same_name['id'],'PUT',{'expected_contact_version':1,'ended':True},expected=409,key=key())
ended=relate(person,same_name,ended=True)
assert ended['ended'] and len(relationships(person))==1
archived=call(orgs+'/'+organization['id']+'/archive','POST',{'expected_version':edited['version'],'archived':True},key=key())
assert archived['archived']
assert relationships(person)[0]['organization_id']==organization['id'], 'Archive must preserve links'
new_person=create_contact('archived link')
call(contacts+'/'+new_person['id']+'/organizations/'+organization['id'],'PUT',{'expected_contact_version':1},expected=409,key=key())
call(orgs+'/'+organization['id']+'/archive','POST',{'expected_version':archived['version'],'archived':False},key=key())

source=create_contact('source')
relate(source,organization,relationship_label='Conflicting',job_title='Director',is_primary=True)
survivor=get_contact(person); source=get_contact(source)
merge={'source_id':source['id'],'source_version':source['version'],'survivor_version':survivor['version'],'resolved':{'display_name':name+' merged'}}
call(contacts+'/'+person['id']+'/merge','POST',merge,expected=409,key=key())
assert get_contact(source)['id']==source['id']
relate(source,organization,relationship_label='Customer',job_title='Director',is_primary=True)
merge['source_version']=get_contact(source)['version']
merged=call(contacts+'/'+person['id']+'/merge','POST',merge,key=key())
assert merged['id']==person['id']
assert len(relationships(person))==1 and relationships(person)[0]['is_primary']
assert relationships(source)==relationships(person), 'Old-ID relationship reads must resolve the survivor'
assert int(sql("SELECT count(*) FROM contacts.contact_organizations WHERE contact_id='"+source['id']+"' AND ended_at IS NOT NULL"))>=2

# Assigned organization visibility also constrains relationship projections.
other_member=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES ('Organization scope fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+fixture.workspace+"',id FROM u RETURNING id").splitlines()[0]
private_org=call(orgs,'POST',{'name':name+' restricted','owner_membership_id':other_member},expected=201,key=key())
relate(person,private_org)
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE permission_key='contacts.organizations.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
try:
    call(orgs+'/'+private_org['id'],expected=404)
    assert not any(r['organization_id']==private_org['id'] for r in relationships(person))
    assert call(orgs+'?search='+name.replace(' ','%20')+'%20restricted&limit=1')['items']==[]
finally:sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE permission_key='contacts.organizations.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")

# Contact version and row lock prevent competing primary assignments.
race=create_contact('race')
def change_primary(org):
    try:
        call(contacts+'/'+race['id']+'/organizations/'+org['id'],'PUT',{'expected_contact_version':1,'is_primary':True},key=key())
        return True
    except AssertionError as failure:
        assert '409' in str(failure);return False
with ThreadPoolExecutor(max_workers=2) as executor:
    assert sum(executor.map(change_primary,[organization,same_name]))==1
assert sum(r['is_primary'] for r in relationships(race))==1
call('/api/v1/workspaces/00000000-0000-0000-0000-000000000001/organizations',expected=403)
print('PASS: organization CRUD/lifecycle, independent identity, temporal relationships, primary concurrency, scoped projections, merge conflicts/history/aliases')

# CLI and MCP share mutations, validation and record projections with HTTP.
cli_org=fixture.cli(['organizations','create','--idempotency-key',key()],{'name':name+' CLI'})
assert call(orgs+'/'+cli_org['id'])==cli_org
mcp_org=fixture.mcp('get_organization',{'workspace_id':fixture.workspace,'organization_id':cli_org['id']})
import json
assert not mcp_org.get('isError') and json.loads(mcp_org['content'][0]['text'])==cli_org
mcp_relation=fixture.mcp('set_contact_organization',{'workspace_id':fixture.workspace,'contact_id':person['id'],'organization_id':cli_org['id'],'idempotency_key':key(),'relationship':{'expected_contact_version':get_contact(person)['version'],'relationship_label':'Partner'}})
assert not mcp_relation.get('isError')
assert fixture.cli(['contacts','organizations',person['id']])==relationships(person)

# Browser forms run without JavaScript and preserve the same relationship rules.
from html.parser import HTMLParser
from urllib.request import Request
from urllib.parse import urlencode
class Fields(HTMLParser):
    def __init__(self):super().__init__();self.values={}
    def handle_starttag(self,tag,attrs):
        attrs=dict(attrs)
        if tag=='input' and attrs.get('type')=='hidden' and attrs.get('name') not in self.values:self.values[attrs['name']]=attrs.get('value','')
def page(path):
    with fixture.browser.open(fixture.base+path,timeout=20) as response:
        html=response.read().decode();fields=Fields();fields.feed(html);return html,fields.values
web='/workspaces/'+fixture.workspace
html,fields=page(web+'/organizations')
assert 'Organizations you work with.' in html
form={**fields,'name':name+' browser','website_url':'https://example.com','notes':'Organization form test'}
with fixture.browser.open(Request(fixture.base+web+'/organizations?/create',data=urlencode(form).encode(),headers={'Origin':fixture.base,'Accept':'text/html'}),timeout=20) as response:
    location=response.geturl();assert 'Organization form test' in response.read().decode()
browser_org=location.rsplit('/',1)[1]
contact=get_contact(person)
form={'organization_id':browser_org,'expected_contact_version':contact['version'],'idempotency_key':key(),'relationship_label':'Browser partner','job_title':'Founder','is_primary':'on'}
with fixture.browser.open(Request(fixture.base+web+'/contacts/'+person['id']+'?/relate',data=urlencode(form).encode(),headers={'Origin':fixture.base,'Accept':'text/html'}),timeout=20) as response:
    html=response.read().decode();assert 'Browser partner' in html and 'Founder' in html
assert next(r for r in relationships(person) if r['organization_id']==browser_org)['is_primary']
with fixture.browser.open(fixture.base+web+'/contacts/'+source['id'],timeout=20) as response:
    assert response.geturl().endswith('/'+person['id']), 'Browser alias must redirect before editing'
print('PASS: organizations and relationship web/API/CLI/MCP parity; browser aliases target editable survivor')

# Database boundaries cannot be bypassed by an accidental unscoped write.
sql("""BEGIN;
DO $test$
DECLARE other uuid; rejected boolean := false;
BEGIN
 INSERT INTO suite.workspaces(name) VALUES('Organization FK isolation') RETURNING id INTO other;
 BEGIN
 INSERT INTO contacts.contact_organizations(workspace_id,contact_id,organization_id)
 VALUES(other,'"""+person['id']+"""','"""+organization['id']+"""');
 EXCEPTION WHEN foreign_key_violation THEN rejected := true;
 END;
 IF NOT rejected THEN RAISE EXCEPTION 'cross workspace relationship accepted'; END IF;
END $test$;
ROLLBACK;""")
# A relationship retry returns its original response; current read authority still applies.
retry_contact=create_contact('retry')
retry_key=key(); retry_input={'expected_contact_version':1,'relationship_label':'Original'}
path=contacts+'/'+retry_contact['id']+'/organizations/'+organization['id']
original=call(path,'PUT',retry_input,key=retry_key)
relate(retry_contact,organization,relationship_label='Changed')
assert call(path,'PUT',retry_input,key=retry_key)==original
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE permission_key='contacts.organizations.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
try:
    private_path=contacts+'/'+retry_contact['id']+'/organizations/'+private_org['id']
    call(private_path,'PUT',{'expected_contact_version':get_contact(retry_contact)['version']},expected=404,key=key())
finally:sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE permission_key='contacts.organizations.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
print('PASS: database workspace foreign keys, original-result relationship retry and authority checks')

preview_person=create_contact('preview')
relate(preview_person,organization,relationship_label='Preview relationship',job_title='Preview title')
preview,_=page(web+'/contacts/'+person['id']+'/merge?source='+preview_person['id'])
assert 'Preview relationship' in preview and 'Preview title' in preview
assert 'Review or resolve relationships' in preview
print('PASS: merge preview includes authorized organization relationships')

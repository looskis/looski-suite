#!/usr/bin/env python3
"""Shared tag catalog and typed assignment journeys against an isolated database."""
import secrets,json
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request
from urllib.parse import urlencode
from html.parser import HTMLParser
from integration_fixture import Fixture
f=Fixture();call,sql=f.call,f.sql
base='/api/v1/workspaces/'+f.workspace
catalog=base+'/tags';contacts=base+'/contacts';orgs=base+'/organizations'
name='Tags '+secrets.token_hex(6)
def key():return secrets.token_hex(16)
def tag(label):return call(catalog,'POST',{'name':name+label,'color':'#123ABC'},expected=201,key=key())
def person(label):return call(contacts,'POST',{'display_name':name+label},expected=201,key=key())
def assign(path,ids,version=None,k=None):return call(path+'/tags','PUT',{'tag_ids':ids,'expected_version':version or call(path)['version']},key=k or key())
call(catalog,expected=401,authenticated=False)
a=tag(' A');b=tag(' B')
call(catalog,'POST',{'name':' '+a['name'].upper()+' '},expected=409,key=key())
call(catalog,'POST',{'name':'Bad','color':'red'},expected=400,key=key())
p=person('person');p_path=contacts+'/'+p['id']
o=call(orgs,'POST',{'name':name+'org'},expected=201,key=key());o_path=orgs+'/'+o['id']
retry=key();original=assign(p_path,[a['id']],1,retry)
assert assign(p_path,[a['id']],1,retry)==original
call(p_path+'/tags','PUT',{'tag_ids':[a['id'],b['id']],'expected_version':1},expected=409,key=key())
call(p_path+'/tags','PUT',{'tag_ids':[a['id'],a['id']],'expected_version':2},expected=400,key=key())
assign(o_path,[a['id']])
renamed=call(catalog+'/'+a['id'],'PATCH',{'name':name+' Renamed','color':'#285b4b','expected_version':a['version']},key=key())
assert call(p_path+'/tags')[0]['name']==renamed['name']
assert call(o_path+'/tags')[0]['id']==a['id']
assert [r['id'] for r in call(contacts+'?tag_id='+a['id'])['items']]==[p['id']]
assert [r['id'] for r in call(orgs+'?tag_id='+a['id'])['items']]==[o['id']]
# Concurrent names and concurrent record edits each admit one winner.
def create_same(_):
 try:tag(' race');return True
 except AssertionError as failure:assert '409' in str(failure);return False
with ThreadPoolExecutor(max_workers=2) as pool:assert sum(pool.map(create_same,range(2)))==1
v=call(p_path)['version']
def set_same(ids):
 try:assign(p_path,ids,v);return True
 except AssertionError as failure:assert '409' in str(failure);return False
with ThreadPoolExecutor(max_workers=2) as pool:assert sum(pool.map(set_same,[[b['id']],[a['id'],b['id']]]))==1
# Merge unions tags and old-ID reads resolve; archived definitions remain attached.
s=person('source');assign(contacts+'/'+s['id'],[a['id']]);assign(p_path,[b['id']])
call(p_path+'/merge','POST',{'source_id':s['id'],'source_version':call(contacts+'/'+s['id'])['version'],'survivor_version':call(p_path)['version'],'resolved':{'display_name':name+'merged'}},key=key())
assert {t['id'] for t in call(p_path+'/tags')}=={a['id'],b['id']}
assert call(contacts+'/'+s['id']+'/tags')==call(p_path+'/tags')
archived=call(catalog+'/'+a['id'],'PATCH',{'name':renamed['name'],'expected_version':renamed['version'],'archived':True},key=key())
assert next(t for t in call(p_path+'/tags') if t['id']==a['id'])['archived']
archived=call(catalog+'/'+a['id'],'PATCH',{'name':renamed['name'],'expected_version':archived['version']},key=key())
assert archived['archived'], 'A rename must not implicitly restore a tag'
new=person('new');new_path=contacts+'/'+new['id']
call(new_path+'/tags','PUT',{'tag_ids':[a['id']],'expected_version':1},expected=409,key=key())
call(catalog,'POST',{'name':renamed['name']},expected=409,key=key())
assign(p_path,[b['id']]);assert not any(t['id']==a['id'] for t in call(p_path+'/tags'))
restored=call(catalog+'/'+a['id'],'PATCH',{'name':renamed['name'],'expected_version':archived['version'],'archived':False},key=key())
assert not restored['archived']
assign(new_path,[a['id']])
# A broad read cannot broaden an assigned tag-mutation grant.
other=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Tag scope fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
hidden=call(contacts,'POST',{'display_name':name+'other','owner_membership_id':other},expected=201,key=key())
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE permission_key='contacts.tags.manage' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
try:call(contacts+'/'+hidden['id']+'/tags','PUT',{'expected_version':1,'tag_ids':[b['id']]},expected=404,key=key())
finally:sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE permission_key='contacts.tags.manage' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
assign(contacts+'/'+hidden['id'],[b['id']])
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE permission_key='contacts.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
try:assert hidden['id'] not in [c['id'] for c in call(contacts+'?tag_id='+b['id'])['items']]
finally:sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE permission_key='contacts.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
# Constraint test uses a real otherwise-valid tag in another workspace.
sql("""BEGIN; DO $check$ DECLARE w uuid; t uuid; rejected boolean:=false; BEGIN
INSERT INTO suite.workspaces(name) VALUES('Tag FK isolation') RETURNING id INTO w;
INSERT INTO suite.tags(workspace_id,name,name_key) VALUES(w,'Other','other') RETURNING id INTO t;
BEGIN INSERT INTO contacts.contact_tags(workspace_id,contact_id,tag_id,created_by_membership_id)
SELECT '"""+f.workspace+"""','"""+p['id']+"""',t,id FROM suite.memberships WHERE workspace_id='"""+f.workspace+"""' LIMIT 1;
EXCEPTION WHEN foreign_key_violation THEN rejected:=true; END;
IF NOT rejected THEN RAISE EXCEPTION 'Cross workspace tag accepted'; END IF;
END $check$; ROLLBACK;""")
cli_tag=f.cli(['tags','create','--idempotency-key',key()],{'name':name+' CLI'})
assert call(catalog+'/'+cli_tag['id'])==cli_tag
mcp=f.mcp('set_contact_tags',{'workspace_id':f.workspace,'contact_id':new['id'],'idempotency_key':key(),'change':{'expected_version':call(new_path)['version'],'tag_ids':[cli_tag['id']]}})
assert not mcp.get('isError'),mcp
assert f.cli(['contacts','tags',new['id']])==call(new_path+'/tags')
assert f.cli(['contacts','list','--tag-id',cli_tag['id']])['items'][0]['id']==new['id']
class Fields(HTMLParser):
 def __init__(self):super().__init__();self.values={}
 def handle_starttag(self,tag,attrs):
  attrs=dict(attrs)
  if tag=='input' and attrs.get('type')=='hidden' and attrs.get('name') not in self.values:self.values[attrs['name']]=attrs.get('value','')
web='/workspaces/'+f.workspace
with f.browser.open(f.base+web+'/tags') as r:
 fields=Fields();fields.feed(r.read().decode())
with f.browser.open(Request(f.base+web+'/tags?/create',data=urlencode({**fields.values,'name':name+' browser','color':'#112233'}).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as r:
 tag_id=r.geturl().rsplit('/',1)[1];assert name+' browser' in r.read().decode()
form={'expected_version':call(new_path)['version'],'tag_id':tag_id,'idempotency_key':key()}
with f.browser.open(Request(f.base+web+'/contacts/'+new['id']+'?/tags',data=urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as r:assert name+' browser' in r.read().decode()
assert [t['id'] for t in call(new_path+'/tags')]==[tag_id]
print('PASS: tag catalog, names/concurrency, rename/link stability, archive/restore rules, assignments/filter scopes, merge aliases, workspace FKs, web/API/CLI/MCP parity')

# Required audit failure rolls the tag assignments and parent version back.
before=call(new_path);before_tags=call(new_path+'/tags')
sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:
 call(new_path+'/tags','PUT',{'tag_ids':[b['id']],'expected_version':before['version']},expected=503,key=key())
finally:sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert call(new_path)['version']==before['version'] and call(new_path+'/tags')==before_tags
# Merge capacity is checked before changing either contact or its relationships.
bulk=sql("INSERT INTO suite.tags(workspace_id,name,name_key) SELECT '"+f.workspace+"','"+name+" capacity '||n,'"+name.lower()+" capacity '||n FROM generate_series(1,51) n RETURNING id").splitlines()
bulk=[identifier for identifier in bulk if len(identifier)==36]
assert len(bulk)==51
full=person('full');overflow=person('overflow')
assign(contacts+'/'+full['id'],bulk[:50]);assign(contacts+'/'+overflow['id'],bulk[50:])
call(contacts+'/'+full['id']+'/merge','POST',{'source_id':overflow['id'],'source_version':2,'survivor_version':2,'resolved':{'display_name':'Capacity merge'}},expected=409,key=key())
assert call(contacts+'/'+overflow['id'])['id']==overflow['id']
assert len(call(contacts+'/'+full['id']+'/tags'))==50
print('PASS: tag/audit transaction rollback and merge capacity preserve both records')

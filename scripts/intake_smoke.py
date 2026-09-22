#!/usr/bin/env python3
"""Automatic identity resolution and retained manual review acceptance."""
import secrets
from concurrent.futures import ThreadPoolExecutor
from integration_fixture import Fixture
f=Fixture();call,sql=f.call,f.sql
base='/api/v1/workspaces/'+f.workspace
intakes=base+'/contact-intakes';contacts=base+'/contacts'
name='Intake '+secrets.token_hex(6)
def key():return secrets.token_hex(16)
def proposed(label,email=None,**extra):return {'display_name':name+label,'identifiers':[] if email is None else [{'kind':'email','value':email}],**extra}
def submit(input,k=None):return call(intakes,'POST',input,expected=201,key=k or key())
def resolve(item,decision,**fields):return call(intakes+'/'+item['id']+'/resolve','POST',{'expected_version':item['version'],'decision':decision,'reason':'Reviewed source evidence',**fields},key=key())
call(intakes,expected=401,authenticated=False)
address=secrets.token_hex(6)+'@example.com'
body={'source_ref':name,'proposed':proposed('first',address,notes='Original input')}
k=key();first=submit(body,k)
assert first['state']=='resolved' and first['contact_id']
assert submit(body,k)==first
person=call(contacts+'/'+first['contact_id']);assert person['notes']=='Original input'
again=submit({'source_ref':name+'again','proposed':proposed('different name',address,notes='Untrusted replacement')})
assert again['contact_id']==person['id']
assert call(contacts+'/'+person['id'])==person,'Unverified intake must not overwrite trusted fields'
assert call(intakes+'/'+again['id'])['input']['proposed']['notes']=='Untrusted replacement'
# Separate requests with the same fresh identifier must choose one contact.
race_address=secrets.token_hex(6)+'@example.com'
with ThreadPoolExecutor(max_workers=2) as pool:
 results=list(pool.map(lambda _:submit({'source_ref':name+'race','proposed':proposed('race',race_address)}),range(2)))
assert len({r['contact_id'] for r in results})==1 and all(r['state']=='resolved' for r in results)
missing=submit({'source_ref':name,'proposed':proposed('missing')});assert missing['state']=='pending'
invalid=submit({'source_ref':name,'proposed':proposed('invalid','not an email')});assert invalid['state']=='pending'
assert call(intakes+'/'+invalid['id'])['input']['proposed']['identifiers'][0]['value']=='not an email'
# Shared addresses do not merge people and cannot resolve automatically.
shared=call(contacts,'POST',proposed('shared',address,allow_shared=True),expected=201,key=key())
ambiguous=submit(body);assert ambiguous['state']=='pending' and ambiguous['contact_id'] is None
linked=resolve(ambiguous,'link',contact_id=person['id']);assert linked['contact_id']==person['id']
call(intakes+'/'+ambiguous['id']+'/resolve','POST',{'expected_version':ambiguous['version'],'decision':'link','reason':'Stale review','contact_id':shared['id']},expected=409,key=key())
# Case-only suggestions remain reviewable; exact local-part identity is preserved.
case_address='Case'+secrets.token_hex(6)+'@example.com'
case=submit({'source_ref':name,'proposed':proposed('case',case_address)})
case_pending=submit({'source_ref':name,'proposed':proposed('case lower',case_address.lower())});assert case_pending['state']=='pending'
case_separate=resolve(case_pending,'create',contact=proposed('case separate',case_address.lower()))
assert case_separate['contact_id']!=case['contact_id']
# Archived matches remain reviewable and cannot silently reactivate.
case_person=call(contacts+'/'+case['contact_id'])
call(contacts+'/'+case_person['id']+'/archive','POST',{'archived':True,'expected_version':case_person['version']},key=key())
assert submit({'source_ref':name,'proposed':proposed('archived',case_address)})['state']=='pending'
# Resolution can explicitly create a provisional person or dismiss an unusable input.
created=resolve(missing,'create',contact=proposed('provisional'))
assert created['state']=='resolved' and created['contact_id']
assert resolve(invalid,'dismiss')['state']=='dismissed'
print('PASS: intake reuse/no overwrite, concurrent resolution, invalid/missing evidence, shared/case/archived review and versioned decisions')

# Visible candidates only, plus record-scoped intake review.
other=sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Intake scope fixture') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
hidden_address=secrets.token_hex(6)+'@example.com'
hidden=call(contacts,'POST',proposed('hidden',hidden_address,owner_membership_id=other),expected=201,key=key())
sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE permission_key='contacts.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
try:
 hidden_intake=submit({'source_ref':name,'proposed':proposed('hidden inquiry',hidden_address)})
 assert hidden_intake['state']=='pending' and hidden_intake['contact_id'] is None
 unmatched=submit({'source_ref':name,'proposed':proposed('unmatched scoped',secrets.token_hex(6)+'@example.com')})
 assert unmatched['state']=='pending' and unmatched['contact_id'] is None, 'Restricted outcomes must not reveal hidden identifier presence'
 for guessed in [hidden_address,secrets.token_hex(6)+'@example.com']:
  call(contacts,'POST',proposed('restricted create',guessed),expected=409,key=key())
  empty=call(contacts,'POST',proposed('empty for scoped edit'),expected=201,key=key())
  call(contacts+'/'+empty['id'],'PATCH',{'expected_version':1,**proposed('edited guess',guessed)},expected=409,key=key())
  merge_source=call(contacts,'POST',proposed('empty merge source'),expected=201,key=key())
  call(contacts+'/'+empty['id']+'/merge','POST',{'source_id':merge_source['id'],'source_version':1,'survivor_version':1,'resolved':proposed('merged guess',guessed)},expected=409,key=key())
 reviewed=call(contacts,'POST',proposed('reviewed separate',hidden_address,allow_shared=True),expected=201,key=key())
 kept=call(contacts+'/'+reviewed['id'],'PATCH',{'expected_version':1,**proposed('retained known identifier',hidden_address)},key=key())
 assert kept['version']==2

 assert {c['id'] for c in call(intakes+'/'+hidden_intake['id']+'/candidates')}=={reviewed['id']}
 call(intakes+'/'+hidden_intake['id']+'/resolve','POST',{'expected_version':1,'decision':'link','contact_id':hidden['id'],'reason':'Unauthorized linking'},expected=404,key=key())
finally:sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE permission_key='contacts.read' AND role_id IN (SELECT id FROM suite.roles WHERE key='owner')")
assert {c['id'] for c in call(intakes+'/'+hidden_intake['id']+'/candidates')}=={hidden['id'],reviewed['id']}
# Never reuse a contact when email and phone resolve to different people.
phone_person=call(contacts,'POST',{'display_name':name+'phone','identifiers':[{'kind':'phone','value':'+1 650 253 0000'}],'allow_shared':True},expected=201,key=key())
conflict=submit({'source_ref':name,'proposed':{'display_name':name+'conflict','identifiers':[{'kind':'email','value':address},{'kind':'phone','value':'+16502530000'}]}})
assert conflict['state']=='pending'
# Input without a name can still reuse a clear identifier; it cannot create an unnamed person.
no_name=submit({'source_ref':name,'proposed':{'display_name':'','identifiers':[{'kind':'email','value':race_address}]}})
assert no_name['state']=='resolved' and no_name['contact_id']==results[0]['contact_id']
# Audit failure rolls back both contact creation and retained intake.
count_before=sql('SELECT count(*) FROM contacts.contacts')
intake_count=sql('SELECT count(*) FROM contacts.intakes')
sql('REVOKE INSERT ON suite.audit_events FROM looski_runtime')
try:call(intakes,'POST',{'source_ref':name,'proposed':proposed('rollback',secrets.token_hex(6)+'@example.com')},expected=503,key=key())
finally:sql('GRANT INSERT ON suite.audit_events TO looski_runtime')
assert sql('SELECT count(*) FROM contacts.contacts')==count_before and sql('SELECT count(*) FROM contacts.intakes')==intake_count
# Original source evidence cannot be rewritten even by an accidental runtime update.
assert sql("SELECT has_column_privilege('looski_runtime','contacts.intakes','input','UPDATE')")=='f'
cli=f.cli(['intakes','submit','--idempotency-key',key()],{'source_ref':name+'CLI','proposed':proposed('CLI')})
assert cli['state']=='pending'
mcp=f.mcp('resolve_contact_intake',{'workspace_id':f.workspace,'intake_id':cli['id'],'idempotency_key':key(),'review':{'expected_version':1,'decision':'dismiss','reason':'Reviewed through MCP'}})
assert not mcp.get('isError'),mcp
assert f.cli(['intakes','get',cli['id']])['state']=='dismissed'
from urllib.request import Request
from urllib.parse import urlencode
from html.parser import HTMLParser
class Fields(HTMLParser):
 def __init__(self):super().__init__();self.values={}
 def handle_starttag(self,tag,attrs):
  attrs=dict(attrs)
  if tag=='input' and attrs.get('type')=='hidden' and attrs.get('name') not in self.values:self.values[attrs['name']]=attrs.get('value','')
web='/workspaces/'+f.workspace
with f.browser.open(f.base+web+'/intakes') as response:
 fields=Fields();fields.feed(response.read().decode())
form={**fields.values,'source_ref':name+'browser','display_name':name+'browser','emails':'','phones':'','notes':'Retained browser evidence'}
with f.browser.open(Request(f.base+web+'/intakes?/submit',data=urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:
 intake_id=response.geturl().rsplit('/',1)[1];assert 'Retained browser evidence' in response.read().decode()
form={'decision':'link','contact_id':person['id'],'reason':'Reviewed browser evidence','expected_version':1,'idempotency_key':key()}
with f.browser.open(Request(f.base+web+'/intakes/'+intake_id,data=urlencode(form).encode(),headers={'Origin':f.base,'Accept':'text/html'})) as response:assert 'Open associated contact' in response.read().decode()
assert call(intakes+'/'+intake_id)['contact_id']==person['id']
print('PASS: hidden candidates, conflicting identities, missing names, immutable evidence, atomic rollback and web/API/CLI/MCP review')

# A case-only suggestion on any unmatched email blocks otherwise exact automatic reuse.
mixed_address='Mixed'+secrets.token_hex(6)+'@example.com'
submit({'source_ref':name,'proposed':proposed('mixed case',mixed_address)})
mixed=submit({'source_ref':name,'proposed':{'display_name':name+'mixed','identifiers':[{'kind':'email','value':race_address},{'kind':'email','value':mixed_address.lower()}]}})
assert mixed['state']=='pending'
# Delay insertion to force both case-variant requests to overlap their decisions.
sql("CREATE FUNCTION contacts.intake_race_delay() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.display_name='"+name+" case race' THEN PERFORM pg_sleep(1); END IF; RETURN NEW; END $$; CREATE TRIGGER intake_race_delay BEFORE INSERT ON contacts.contacts FOR EACH ROW EXECUTE FUNCTION contacts.intake_race_delay();")
try:
 case_race='Concurrent'+secrets.token_hex(6)+'@example.com'
 with ThreadPoolExecutor(max_workers=2) as pool:
  case_results=list(pool.map(lambda address:submit({'source_ref':name,'proposed':proposed(' case race',address)}),[case_race,case_race.lower()]))
 assert sorted(r['state'] for r in case_results)==['pending','resolved']
finally:sql('DROP TRIGGER intake_race_delay ON contacts.contacts; DROP FUNCTION contacts.intake_race_delay();')
print('PASS: non-disclosing restricted intake, mixed exact/case-only ambiguity and serialized case-variant races')

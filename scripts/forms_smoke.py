#!/usr/bin/env python3
"""Forms: versioned definitions, conditional public submissions, notifications and the
Forms-to-CRM inquiry recipe (Milestone 2 acceptance) on an isolated installation."""
import json
import secrets
import subprocess
import time
from integration_fixture import Fixture
f=Fixture();call,sql=f.call,f.sql
base='/api/v1/workspaces/'+f.workspace
name='Forms '+secrets.token_hex(6)
def key():return secrets.token_hex(16)
def module(k,enabled):
 m=next(m for m in call(base+'/modules') if m['key']==k)
 if m['enabled']!=enabled:m=call(base+'/modules/'+k,'PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
 return m
def wait_for(predicate,what,seconds=30):
 deadline=time.monotonic()+seconds
 while True:
  value=predicate()
  if value:return value
  assert time.monotonic()<deadline,'Timed out waiting for '+what
  time.sleep(.2)
fields=[
 {'key':'first_name','label':'First name','kind':'short_text','required':True},
 {'key':'last_name','label':'Last name','kind':'short_text'},
 {'key':'email','label':'Email','kind':'email'},
 {'key':'phone','label':'Phone','kind':'phone'},
 {'key':'topic','label':'Topic','kind':'choice','required':True,'options':[{'key':'sales','label':'Sales'},{'key':'support','label':'Support'}]},
 {'key':'budget','label':'Budget','kind':'number','required':True,'show_if':{'field':'topic','equals':['sales']}},
 {'key':'consent','label':'You may contact me','kind':'checkbox','required':True},
]
# Forms works with CRM disabled: nothing before the recipe section needs it.
module('crm',False)
# Disabled Forms closes staff routes and public endpoints; records survive disablement.
module('forms',False)
call(base+'/forms',expected=403)
module('forms',True)
call(base+'/forms','POST',{'name':name,'draft':{'fields':[{'key':'Bad Key','label':'x','kind':'short_text'}]}},expected=400,key=key())
retry=key();form=call(base+'/forms','POST',{'name':name,'description':'Tell us about your project','confirmation_message':'Thanks, we will be in touch.','draft':{'fields':fields[:2]}},expected=201,key=retry)
assert call(base+'/forms','POST',{'name':name,'description':'Tell us about your project','confirmation_message':'Thanks, we will be in touch.','draft':{'fields':fields[:2]}},expected=201,key=retry)==form
path=base+'/forms/'+form['id'];public='/api/v1/public/forms/'+form['public_key']
assert form['published_version'] is None and not form['accepting']
call(public,expected=404,authenticated=False)
form=call(path,'PATCH',{'name':name,'description':form['description'],'confirmation_message':form['confirmation_message'],'draft':{'fields':fields},'expected_version':form['version']},key=key())
call(path,'PATCH',{'name':name,'draft':{'fields':fields},'expected_version':1},expected=409,key=key())
form=call(path+'/publish','POST',{'expected_version':form['version']},key=key())
assert form['published_version']['number']==1 and form['accepting'] and not form['unpublished_changes']
call(path+'/publish','POST',{'expected_version':form['version']},expected=409,key=key())
definition=call(public,authenticated=False)
assert set(definition)=={'name','description','definition'},'Public reads expose only the published form'
assert definition['name']==name and [f['key'] for f in definition['definition']['fields']]==[f['key'] for f in fields]
call('/api/v1/public/forms/'+'x'*22,expected=404,authenticated=False)
call('/api/v1/public/forms/%00'+'x'*21,expected=404,authenticated=False)
def client():
 # The gateway trusts CF-Connecting-IP from private peers, as behind the tunnel.
 return {'CF-Connecting-IP':'198.51.100.'+str(secrets.randbelow(250)+1)}
def submit(answers,expected=201,submission_key=None,headers=None):
 return call(public+'/submissions','POST',{'submission_key':submission_key or secrets.token_hex(12),'answers':answers},expected=expected,authenticated=False,headers=headers or client())
ada={'first_name':' Ada ','last_name':'Lovelace','email':'ada.'+secrets.token_hex(4)+'@example.com','topic':'sales','budget':5000,'consent':True}
submit({**ada,'budget':None},400)  # Required conditional question.
submit({**ada,'topic':'support'},400)  # Answer to a hidden question.
submit({**ada,'consent':False},400)
submit({**ada,'surprise':'x'},400)
once=secrets.token_hex(12)
assert submit(ada,submission_key=once)['confirmation_message']=='Thanks, we will be in touch.'
submit(ada,submission_key=once)  # A retried submission is not duplicated.
submit({**ada,'budget':1},409,submission_key=once)
page=call(path+'/submissions')
assert len(page['items'])==1 and page['items'][0]['version_number']==1
assert page['items'][0]['answers']=={**ada,'first_name':'Ada'} and page['items'][0]['recipe_run'] is None
# Retrying a committed submission succeeds after the form closes and after a new version.
closing=call(path)
closing=call(path+'/accepting','POST',{'expected_version':closing['version'],'accepting':False},key=key())
assert submit(ada,submission_key=once)['confirmation_message']=='Thanks, we will be in touch.'
submit({**ada,'budget':1},409,submission_key=once)
form=call(path+'/accepting','POST',{'expected_version':closing['version'],'accepting':True},key=key())
assert call(path)['submission_count']==1
# A new version leaves earlier submissions on the version they answered.
form=call(path,'PATCH',{'name':name,'description':form['description'],'confirmation_message':form['confirmation_message'],'draft':{'fields':fields+[{'key':'source','label':'How did you hear about us?','kind':'long_text'}]},'expected_version':form['version']},key=key())
assert form['unpublished_changes'] and form['published_version']['number']==1
form=call(path+'/publish','POST',{'expected_version':form['version']},key=key())
assert form['published_version']['number']==2 and [v['number'] for v in call(path+'/versions')]==[2,1]
submit({'first_name':'Grace','topic':'support','consent':True,'source':'A friend'})
items=call(path+'/submissions')['items']
assert [s['version_number'] for s in items]==[2,1]
assert submit(ada,submission_key=once)['confirmation_message'] and call(path)['submission_count']==2,'A retry is judged against the version it answered'
summary=call(path+'/summary')
topic=next(s for s in summary['fields'] if s['key']=='topic')
assert summary['submissions']==2 and {o['key']:o['count'] for o in topic['options']}=={'sales':1,'support':1}
# Closing stops public answers without exposing the form; reopening restores them.
form=call(path+'/accepting','POST',{'expected_version':form['version'],'accepting':False},key=key())
call(public,expected=409,authenticated=False);submit(ada,409)
form=call(path+'/accepting','POST',{'expected_version':form['version'],'accepting':True},key=key())
# Public rights are specific to the form: the key opens nothing else.
call(base+'/forms',expected=401,authenticated=False)
call(path+'/submissions',expected=401,authenticated=False)
# One client is throttled before submissions reach the database.
flood=client();statuses=[]
for _ in range(25):
 try:submit({'first_name':'x'},headers=flood)
 except AssertionError as failure:statuses.append(429 if '429' in str(failure) else 400)
assert 429 in statuses,statuses
# Business export: complete forms, immutable versions and original answers, with their own permissions.
def export(collection,expected=200,**query):return call(base+'/exports/page','POST',{'collection':collection,**query},expected=expected)
catalog={c['collection']:c for c in call(base+'/exports/collections')}
assert catalog['forms']['read_permission']=='forms.read' and not catalog['forms']['parent_required'] and catalog['forms']['supports_archive_selection']
assert catalog['form_versions']['read_permission']=='forms.read' and catalog['form_versions']['parent_required']
assert catalog['form_submissions']['read_permission']=='forms.submissions.read' and catalog['form_submissions']['dependent_read_permissions']==['forms.read']
def every(collection,**query):
 items=[];page=export(collection,limit=1,**query)
 while True:
  items+=page['items']
  if not page['next']:return items
  page=call(base+'/exports/page','POST',page['next'])
exported=next(x for x in every('forms') if x['id']==form['id'])
assert exported==call(path) and exported['draft']['fields']
assert every('form_versions',parent_id=form['id'])==call(path+'/versions')
assert every('form_submissions',parent_id=form['id'])==items,'Export keeps original answers and their version'
export('form_submissions',400)
export('form_versions',404,parent_id='00000000-0000-0000-0000-000000000000')
assert f.cli(['export','page'],{'collection':'form_versions','parent_id':form['id'],'limit':1})==export('form_versions',parent_id=form['id'],limit=1)
owner_role="(SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner')"
for permission,collection in [('forms.submissions.read','form_submissions'),('forms.read','form_submissions'),('forms.read','form_versions'),('forms.read','forms')]:
 sql("DELETE FROM suite.role_permissions WHERE role_id="+owner_role+" AND permission_key='"+permission+"'")
 try:export(collection,403,**({} if collection=='forms' else {'parent_id':form['id']}))
 finally:sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"',"+owner_role+",'"+permission+"')")
module('forms',False)
export('forms',403);export('form_submissions',403,parent_id=form['id'])
module('forms',True)
print('PASS: versioned definitions, conditional validation, idempotent public submissions, close/reopen, summary, business export with CRM disabled')

# Forms-to-CRM inquiry recipe. Configuration needs CRM; collecting answers never did.
call(path+'/recipe','PUT',{'mapping':{'email_field':'email','pipeline_id':f.workspace,'stage_id':f.workspace,'title_prefix':'x'}},expected=403,key=key())
crm=module('crm',True)
def definition_record(kind,**body):return call(base+'/crm/'+kind,'POST',{'name':name,**body},expected=201,key=key())
pipeline=definition_record('pipelines');stage=definition_record('stages',parent_id=pipeline['id'],outcome='open')
mapping={'name_fields':['first_name','last_name'],'email_field':'email','phone_field':'phone','phone_country':'US','pipeline_id':pipeline['id'],'stage_id':stage['id'],'title_prefix':'Website inquiry'}
call(path+'/recipe',expected=404)
call(path+'/recipe','PUT',{'mapping':{**mapping,'email_field':'first_name'}},expected=400,key=key())
recipe=call(path+'/recipe','PUT',{'mapping':mapping},key=key())
assert recipe['revision']['number']==1 and not recipe['active']
recipe=call(path+'/recipe/activation','POST',{'expected_version':recipe['version'],'active':True},key=key())
def run_for(submission_key):
 sid=sql("SELECT id FROM forms.submissions WHERE submission_key='"+submission_key+"'")
 return next((s['recipe_run'] for s in call(path+'/submissions?limit=100')['items'] if s['id']==sid),None)
def settled(submission_key,states=('completed',),seconds=30):
 return wait_for(lambda:(lambda r:r if r and (r['state'] in states or r['job_state'] in ('blocked','exhausted')) else None)(run_for(submission_key)),'recipe run',seconds)
email='inquiry.'+secrets.token_hex(4)+'@example.com'
first=secrets.token_hex(12);submit({**ada,'email':email},submission_key=first)
run=settled(first)
assert run['state']=='completed' and run['contact_id'] and run['opportunity_id'],run
# Retrying the completed submission repeats no recipe effect.
submit({**ada,'email':email},submission_key=first)
time.sleep(1)
assert run_for(first)==run
assert sql("SELECT count(*) FROM crm.opportunities WHERE pipeline_id='"+pipeline['id']+"'")=='1' and sql("SELECT count(*) FROM forms.crm_recipe_runs WHERE recipe_id='"+recipe['id']+"'")=='1'
opportunity=call(base+'/crm/opportunities/'+run['opportunity_id'])
assert opportunity['title']=='Website inquiry: Ada Lovelace' and opportunity['pipeline_id']==pipeline['id']
participants=call(base+'/crm/opportunities/'+run['opportunity_id']+'/contacts')['items']
assert [p['reference_id'] for p in participants]==[run['contact_id']] and participants[0]['is_primary']
contact=call(base+'/contacts/'+run['contact_id'])
assert contact['display_name']=='Ada Lovelace'
# Staff-curated data is what public input must not overwrite: names, notes and identifiers alike.
contact=call(base+'/contacts/'+run['contact_id'],'PATCH',{'display_name':'Ada (verified)','notes':'Trusted staff note','identifiers':[{'kind':i['kind'],'value':i['value'],'country':None,'extension':None} for i in contact['identifiers']],'owner_membership_id':contact['owner_membership_id'],'expected_version':contact['version']},key=key())
# A genuinely new submission by the same person reuses the contact and records a new inquiry.
second=secrets.token_hex(12);submit({**ada,'first_name':'Augusta','email':email},submission_key=second)
again=settled(second)
assert again['contact_id']==run['contact_id'] and again['opportunity_id']!=run['opportunity_id']
assert call(base+'/contacts/'+run['contact_id'])==contact,'Public input must not overwrite trusted contact data'
assert sql("SELECT count(*) FROM forms.crm_recipe_runs WHERE submission_id IN (SELECT id FROM forms.submissions WHERE submission_key IN ('"+first+"','"+second+"'))")=='2'
# Conflicting identifiers stop for visible review; retry continues after resolution.
phone_owner=call(base+'/contacts','POST',{'display_name':name+' phone owner','identifiers':[{'kind':'phone','value':'+1 415 555 0'+str(secrets.randbelow(900)+100),'country':'US'}]},expected=201,key=key())
phone=phone_owner['identifiers'][0]['value']
ambiguous=secrets.token_hex(12);submit({**ada,'email':email,'phone':phone},submission_key=ambiguous)
review=settled(ambiguous,('awaiting_review',))
assert review['state']=='awaiting_review' and review['opportunity_id'] is None and review['intake_id']
intake=call(base+'/contact-intakes/'+review['intake_id'])
assert intake['state']=='pending'
call(base+'/contact-intakes/'+review['intake_id']+'/resolve','POST',{'expected_version':intake['version'],'decision':'link','reason':'Same person, new phone','contact_id':run['contact_id']},key=key())
review=call(path+'/recipe/runs/'+review['id']+'/retry','POST',{'expected_version':review['version']},key=key())
done=settled(ambiguous)
assert done['state']=='completed' and done['contact_id']==run['contact_id']
call(path+'/recipe/runs/'+done['id']+'/retry','POST',{'expected_version':done['version']},expected=409,key=key())
print('PASS: recipe mapping, contact resolution/reuse, independent inquiries, visible review and explicit retry')

# A mapping that no longer fits the answered version is a visible failure with answers intact.
current=call(path)
call(path,'PATCH',{'name':name,'description':current['description'],'confirmation_message':current['confirmation_message'],'draft':{'fields':[x for x in current['draft']['fields'] if x['key']!='phone']},'expected_version':current['version']},key=key())
current=call(path+'/publish','POST',{'expected_version':current['version']+1},key=key())
unmapped=secrets.token_hex(12);submit({**ada,'email':'unmapped.'+secrets.token_hex(4)+'@example.com'},submission_key=unmapped)
invalid=settled(unmapped)
assert invalid['job_state']=='blocked' and invalid['error_code']=='invalid_input' and invalid['opportunity_id'] is None,invalid
assert sql("SELECT count(*) FROM forms.submissions WHERE submission_key='"+unmapped+"'")=='1'
call(path,'PATCH',{'name':name,'description':current['description'],'confirmation_message':current['confirmation_message'],'draft':{'fields':fields},'expected_version':current['version']},key=key())
call(path+'/publish','POST',{'expected_version':current['version']+1},key=key())
# A failing step rolls back and retries without duplicating completed effects.
sql("""CREATE OR REPLACE FUNCTION crm.forms_probe_fail() RETURNS trigger LANGUAGE plpgsql AS $$
 BEGIN IF NEW.title LIKE 'Website inquiry: Fault%' THEN RAISE EXCEPTION 'injected fault'; END IF; RETURN NEW; END $$;
 CREATE TRIGGER forms_probe_fail BEFORE INSERT ON crm.opportunities FOR EACH ROW EXECUTE FUNCTION crm.forms_probe_fail();""")
try:
 fault=secrets.token_hex(12);fault_name='Fault'+secrets.token_hex(4)
 submit({**ada,'first_name':fault_name,'email':'fault.'+secrets.token_hex(4)+'@example.com'},submission_key=fault)
 failed=settled(fault,seconds=90)  # Bounded backoff over five attempts.
 assert failed['job_state']=='exhausted' and failed['error_code']=='database',failed
 assert failed['intake_id'] is None and failed['opportunity_id'] is None,'Failed steps must not partially commit'
finally:sql("DROP TRIGGER forms_probe_fail ON crm.opportunities; DROP FUNCTION crm.forms_probe_fail();")
call(path+'/recipe/runs/'+failed['id']+'/retry','POST',{'expected_version':failed['version']},key=key())
recovered=settled(fault)
assert recovered['state']=='completed'
assert sql("SELECT count(*) FROM crm.opportunities WHERE title='Website inquiry: "+fault_name+" Lovelace'")=='1'
# Missing permissions and a disabled CRM are visible failures, never bypassed.
sql("DELETE FROM suite.role_permissions WHERE role_id="+owner_role+" AND permission_key='crm.opportunities.create'")
try:
 denied=secrets.token_hex(12);submit({**ada,'email':'denied.'+secrets.token_hex(4)+'@example.com'},submission_key=denied)
 blocked=settled(denied)
 assert blocked['job_state']=='blocked' and blocked['error_code']=='authority',blocked
finally:sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"',"+owner_role+",'crm.opportunities.create')")
call(path+'/recipe/runs/'+blocked['id']+'/retry','POST',{'expected_version':blocked['version']},key=key())
assert settled(denied)['state']=='completed'
module('crm',False)
paused=secrets.token_hex(12);submit({**ada,'email':'paused.'+secrets.token_hex(4)+'@example.com'},submission_key=paused)
held=settled(paused)
assert held['job_state']=='blocked' and held['error_code']=='module_disabled' and held['opportunity_id'] is None,held
call(path+'/recipe/runs/'+held['id']+'/retry','POST',{'expected_version':held['version']},expected=409,key=key())
module('crm',True)
call(path+'/recipe/runs/'+held['id']+'/retry','POST',{'expected_version':held['version']},key=key())
assert settled(paused)['state']=='completed'
# A worker outage after the submission commit leaves durable pending work.
subprocess.run(['docker','compose','stop','worker'],cwd=f.directory.parents[1],env=f.environment,check=True,capture_output=True)
try:
 outage=secrets.token_hex(12);submit({**ada,'email':'outage.'+secrets.token_hex(4)+'@example.com'},submission_key=outage)
 time.sleep(1)
 pending=run_for(outage);assert pending['state']=='queued'
 # A worker that crashed mid-run leaves an expired lease; the job is claimed again.
 sql("UPDATE suite.jobs SET state='running',attempts=1,lease_token=gen_random_uuid(),lease_until=now()-interval '1 second' WHERE id='"+pending['job_id']+"'")
finally:subprocess.run(['docker','compose','start','worker'],cwd=f.directory.parents[1],env=f.environment,check=True,capture_output=True)
assert settled(outage,seconds=90)['state']=='completed'
assert sql("SELECT count(*) FROM forms.crm_recipe_runs WHERE id='"+pending['id']+"' AND opportunity_id IS NOT NULL")=='1'
# Deactivation stops new runs without discarding submissions.
recipe=call(path+'/recipe')
recipe=call(path+'/recipe/activation','POST',{'expected_version':recipe['version'],'active':False},key=key())
quiet=secrets.token_hex(12);submit({**ada,'email':'quiet.'+secrets.token_hex(4)+'@example.com'},submission_key=quiet)
assert run_for(quiet) is None
print('PASS: rollback/retry without duplicates, authority and module blocks, worker outage durability, deactivation')

# Notifications go only to members who can read submissions, through SMTP.
if f.config['LOOSKI_MAIL_MODE']=='local':
 owner_member=sql("SELECT m.id FROM suite.memberships m JOIN suite.membership_roles mr ON mr.membership_id=m.id WHERE m.workspace_id='"+f.workspace+"' AND mr.role_id="+owner_role+" LIMIT 1")
 form=call(path)
 form=call(path+'/notifications','PUT',{'expected_version':form['version'],'membership_ids':[owner_member]},key=key())
 assert form['notification_membership_ids']==[owner_member]
 owner=json.loads((f.directory/'identity-fixture.json').read_text())
 def inbox():
  data=subprocess.check_output(['docker','compose','exec','-T','auth','wget','-qO-','http://mailpit:8025/api/v1/messages'],cwd=f.directory.parents[1],env=f.environment)
  return [m for m in json.loads(data)['messages'] if any(t['Address']==owner['email'] for t in m['To']) and m['Subject']=='New response: '+name]
 before=len(inbox())
 submit({**ada,'email':'notify.'+secrets.token_hex(4)+'@example.com'})
 wait_for(lambda:len(inbox())>before,'notification email')
 message=inbox()[0]
 assert 'ada' not in json.dumps(message).lower(),'Answers stay in the suite'
 form=call(path+'/notifications','PUT',{'expected_version':form['version'],'membership_ids':[]},key=key())
 print('PASS: submission notification delivered over SMTP without answer content')

# A contact recorded by a recipe run is retained, not erased from under its inquiry.
held_contact=call(base+'/contacts/'+run['contact_id'])
held_contact=call(base+'/contacts/'+held_contact['id']+'/archive','POST',{'expected_version':held_contact['version'],'archived':True},key=key())
call(base+'/contacts/'+held_contact['id']+'/delete-permanently','POST',{'expected_version':held_contact['version'],'confirm_id':held_contact['id']},expected=409,key=key())
# CLI and MCP use the same operations.
cli_forms=f.cli(['forms','list','--search',name])
assert [x['id'] for x in cli_forms['items']]==[form['id']]
mcp=f.mcp('get_form',{'workspace_id':f.workspace,'form_id':form['id']})
assert json.loads(mcp['content'][0]['text'])['id']==form['id']
# Disabling Forms closes the public form and retains every record.
module('forms',False)
call(public,expected=404,authenticated=False);submit(ada,404)
module('forms',True)
# Answers past the stored limit are a visible rejection, not a storage failure.
large=[{'key':'q'+str(i),'label':'Long answer','kind':'long_text'} for i in range(20)]
sized=call(base+'/forms','POST',{'name':name+' sized','draft':{'fields':large}},expected=201,key=key())
sized=call(base+'/forms/'+sized['id']+'/publish','POST',{'expected_version':sized['version']},key=key())
call('/api/v1/public/forms/'+sized['public_key']+'/submissions','POST',{'submission_key':secrets.token_hex(12),'answers':{f['key']:'x'*10000 for f in large}},expected=400,authenticated=False,headers=client())
call(base+'/forms','POST',{'name':name+' big','draft':{'fields':[{'key':'q'+str(i),'label':'x'*500,'help':'y'*1000,'kind':'choice','options':[{'key':'o'+str(o),'label':'z'*200} for o in range(50)]} for i in range(100)]}},expected=400,key=key())
assert call(path)['submission_count']>=10
print('PASS: forms CLI/MCP parity and module disable/re-enable retention')

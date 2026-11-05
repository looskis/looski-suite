#!/usr/bin/env python3
"""Record field/view acceptance against the dedicated identity-check installation."""
import json,uuid
from urllib.parse import urlencode
from integration_fixture import Fixture
f=Fixture();root='/api/v1/workspaces/'+f.workspace;prefix='Records '+uuid.uuid4().hex[:8]
def call(path,method='GET',body=None,expected=200,key=None):return f.call(root+path,method,body,expected,key or (str(uuid.uuid4()) if method!='GET' else None))
def cond(field,op,value=None):return {'type':'condition','field':field,'operator':op,'value':value}
def group(op,*rules):return {'type':'group','operator':op,'rules':list(rules)}
def listing(kind,expression=None,**query):return call('/'+kind+'?'+urlencode({'search':prefix,**query,**({'filter':json.dumps(expression)} if expression else {})}))
fieldkey='custom_score_'+uuid.uuid4().hex[:8]
field=call('/records/contacts/fields','POST',{'key':fieldkey,'name':'Fit score','field_type':'number','options':[]})
assert any(v['key']==fieldkey for v in call('/records/contacts/fields'))
contacts=[]
for index,score in enumerate([0,10,10,20,None]):
 body={'display_name':prefix+str(index),'identifiers':[{'kind':'email','value':f'z{index}.{uuid.uuid4().hex[:8]}@example.com'},{'kind':'email','value':f'a{index}.{uuid.uuid4().hex[:8]}@example.com'}],'notes':'','properties':{'status':'customer' if index==3 else 'lead','job_title':'Director',fieldkey:score,'location':{'country_code':'CA','postal_code':'H2X 1Y4','locality':'Montréal'}}}
 record=call('/contacts','POST',body,201);assert record['identifiers'][0]['value']==body['identifiers'][0]['value'];contacts.append(record)
expression=group('and',cond(fieldkey,'gte',10),group('or',cond('status','eq','customer'),cond(fieldkey,'eq',10)))
assert len(listing('contacts',expression)['items'])==3
assert len(listing('contacts',cond('primary_email','contains','@example.com'))['items'])==5
assert len(listing('contacts',cond(fieldkey,'lt',10))['items'])==1 # null is not zero
assert len(listing('contacts',cond(fieldkey,'is_empty'))['items'])==1
assert len(listing('contacts',cond('name','contains',"' OR TRUE --"))['items'])==0
for direction in ['asc','desc']:
 after=None;seen=[]
 while True:
  result=listing('contacts',sort=fieldkey,direction=direction,limit=2,**({'after':after} if after else {}));seen.extend(result['items']);after=result['next_cursor']
  if not after:break
 assert len(seen)==5 and len({r['id'] for r in seen})==5
 assert seen[-1]['properties'][fieldkey] is None
 assert [r['properties'][fieldkey] for r in seen[:-1]]==sorted([0,10,10,20],reverse=direction=='desc')
print('PASS: nested typed filters, substring email matching, null semantics and stable numeric pagination')
original=contacts[0];legacy={'display_name':original['display_name'],'notes':'Legacy update','identifiers':[{'kind':i['kind'],'value':i['value']} for i in original['identifiers']],'expected_version':original['version']}
updated=call('/contacts/'+original['id'],'PATCH',legacy);assert updated['properties']==original['properties']
for properties in [{'location':{'country_code':'ZZ'}},{'location':{'locality':'London'}},{'unknown':'bad'},{fieldkey:'invalid'}]:
 call('/contacts/'+original['id'],'PATCH',{**legacy,'expected_version':updated['version'],'properties':properties},400)
org=call('/organizations','POST',{'name':prefix+' Org','website_url':'https://example.com','notes':'','properties':{'industry':'Technology','employee_count':25,'annual_revenue':1200000,'location':{'country_code':'JP','postal_code':'100-0001','locality':'千代田区'}}},201)
assert listing('organizations',cond('employee_count','gte',20))['items'][0]['id']==org['id']
print('PASS: primary email order, backward-compatible updates and international address validation')
config={'columns':['name','primary_email',fieldkey],'filter':expression,'sort':fieldkey,'direction':'desc','archived':False}
body={'name':prefix+' view','shared':True,'config':config};key=str(uuid.uuid4())
view=call('/records/contacts/views','POST',body,key=key);assert call('/records/contacts/views','POST',body,key=key)==view
assert any(v['id']==view['id'] for v in call('/records/contacts/views'))
changed=call('/records/contacts/views/'+view['id'],'PUT',{**body,'name':prefix+' renamed','expected_version':view['version']})
call('/records/contacts/views/'+view['id'],'PUT',{**body,'expected_version':view['version']},409)
call('/records/contacts/fields','POST',{**field,'field_type':'text','expected_version':field['version']},400) # unknown DTO keys rejected
call('/records/contacts/fields','POST',{'key':fieldkey,'name':'Fit score','field_type':'text','options':[],'expected_version':field['version']},400)
call('/records/contacts/fields','POST',{'key':fieldkey,'name':'Fit score','field_type':'number','options':[],'archived':True,'expected_version':field['version']})
assert len(listing('contacts',expression)['items'])==3
call('/records/contacts/views/'+view['id'],'DELETE',{'expected_version':changed['version']})
assert all(v['id']!=view['id'] for v in call('/records/contacts/views'))
print('PASS: view replay, optimistic concurrency, field type stability, archive preservation and deletion')
# Denied workspace and anonymous callers never get metadata or records.
f.call('/api/v1/workspaces/'+str(uuid.uuid4())+'/records/contacts/fields',expected=403)
f.call(root+'/records/contacts/views',authenticated=False,expected=401)
print('PASS: workspace authorization and unauthenticated denial')
# Frequency is explicit, validated, and retained by legacy edits.
crm=next(m for m in call('/modules') if m['key']=='crm')
if not crm['enabled']:call('/modules/crm','PATCH',{'enabled':True,'expected_version':crm['version']})
pipeline=call('/crm/pipelines','POST',{'name':prefix+' pipeline'},201)
stages=[call('/crm/stages','POST',{'name':'Qualified','parent_id':pipeline['id'],'outcome':'open'},201)]
oppbody={'title':prefix+' annual','notes':'','pipeline_id':pipeline['id'],'stage_id':stages[0]['id'],'expected_value':'60000','currency_code':'USD','billing_frequency':'annually'}
opp=call('/crm/opportunities','POST',oppbody,201);assert opp['billing_frequency']=='annually'
call('/crm/opportunities','POST',{**oppbody,'billing_frequency':'weekly'},400)
legacy={k:v for k,v in oppbody.items() if k!='billing_frequency'}
opp=call('/crm/opportunities/'+opp['id'],'PATCH',{**legacy,'expected_version':opp['version']});assert opp['billing_frequency']=='annually'
print('PASS: opportunity frequency validation and backward compatibility')

category=call('/crm/categories','POST',{'name':prefix+' category'},201)
statuses=[call('/crm/statuses','POST',{'name':'Active','parent_id':category['id']},201)]
dealbody={'title':prefix+' monthly','notes':'','category_id':category['id'],'status_id':statuses[0]['id'],'value':'5000','currency_code':'USD','billing_frequency':'monthly'}
deal=call('/crm/deals','POST',dealbody,201);assert deal['billing_frequency']=='monthly'
legacy={k:v for k,v in dealbody.items() if k!='billing_frequency'}
deal=call('/crm/deals/'+deal['id'],'PATCH',{**legacy,'expected_version':deal['version']});assert deal['billing_frequency']=='monthly'
print('PASS: deal frequency and backward-compatible updates')

#!/usr/bin/env python3
"""Explicit starter copies, rollback, safe retirement and all-interface parity."""
import secrets, subprocess, json
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request,urlopen
from urllib.error import HTTPError
from urllib.parse import urlencode,urlparse
from integration_fixture import Fixture
from browser_forms import Forms
from dev import ROOT
f=Fixture();call=f.call;base='/api/v1/workspaces/'+f.workspace;path=base+'/crm/starter-packages'
def key():return secrets.token_hex(16)
def create(route,body):return call(base+route,'POST',body,expected=201,key=key())
def install():return call(path,'POST',{'package_version':1},expected=201,key=key())
def remove(item,k=None,expected=200):return call(path+'/'+item['id']+'/remove','POST',{'expected_version':item['version']},expected=expected,key=k or key())
def record(item,kind,name=None):return next(r for r in item['records'] if r['kind']==kind and (name is None or r['name']==name))
def rid(item,kind,name=None):return record(item,kind,name)['id']
def config_path(r):return base+('/tasks/types/' if r['kind']=='task-types' else '/tasks/templates/' if r['kind']=='task-templates' else '/crm/'+r['kind']+'/')+r['id']
def counts():return f.sql("SELECT (SELECT count(*) FROM crm.starter_installations),(SELECT count(*) FROM crm.opportunity_pipelines),(SELECT count(*) FROM crm.opportunity_stages),(SELECT count(*) FROM crm.deal_categories),(SELECT count(*) FROM crm.deal_statuses),(SELECT count(*) FROM crm.activity_types),(SELECT count(*) FROM tasks.task_types),(SELECT count(*) FROM tasks.templates),(SELECT count(*) FROM tasks.template_revisions),(SELECT count(*) FROM crm.coordination_templates),(SELECT count(*) FROM crm.coordination_rules),(SELECT count(*) FROM suite.audit_events),(SELECT count(*) FROM suite.idempotency_records)")
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if not module['enabled']:call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key())
# Remove any active package left by an interrupted run without touching business records.
active=f.sql("SELECT id FROM crm.starter_installations WHERE workspace_id='"+f.workspace+"' AND removed_at IS NULL")
if active:remove(call(path+'/'+active))
assert call(path)['package']['version']==1
call(path+'?limit=101',expected=400);call(path,'POST',{'package_version':2},expected=400,key=key())
before=counts();failed=key()
f.sql('REVOKE INSERT ON crm.starter_statuses FROM looski_runtime')
try:call(path,'POST',{'package_version':1},expected=503,key=failed)
finally:f.sql('GRANT INSERT ON crm.starter_statuses TO looski_runtime')
assert counts()==before
first=call(path,'POST',{'package_version':1},expected=201,key=failed)
assert len(first['records'])==16 and all(r['version']==1 and not r['archived'] for r in first['records'])
assert call(path,'POST',{'package_version':1},expected=201,key=failed)==first
call(path,'POST',{'package_version':1},expected=409,key=key())
assert call(path+'/'+first['id'])==first
print('PASS: explicit versioned installation, late rollback and stable retry without duplicate definitions')
# The package contains ordinary editable definitions; real work outlives removal.
name='Starter '+key()[:8]
opp=create('/crm/opportunities',{'title':name,'pipeline_id':rid(first,'pipelines'),'stage_id':rid(first,'stages','New')})
deal=create('/crm/deals',{'title':name,'category_id':rid(first,'categories'),'status_id':rid(first,'statuses','Planned'),'originating_opportunity_id':opp['id']})
activity=create('/crm/activities',{'type_id':rid(first,'activity-types','Note'),'anchor_kind':'deal','anchor_id':deal['id'],'subject':name,'body':'Preserve this evidence','occurred_at':'2028-01-01T00:00:00Z'})
coord=call(config_path(record(first,'coordination-templates')))
instance=create('/crm/deals/'+deal['id']+'/coordination',{'expected_version':deal['version'],'revision_id':coord['revision']['id'],'base_on':'2028-01-01'})
task=instance['items'][0]['task']
subprocess.run(['docker','compose','stop','worker'],cwd=ROOT,env=f.environment,check=True,capture_output=True)
try:
 current=call(base+'/crm/deals/'+deal['id'])
 call(base+'/crm/deals/'+deal['id'],'PATCH',{'title':name,'category_id':rid(first,'categories'),'status_id':rid(first,'statuses','Active'),'expected_version':current['version']},key=key())
 job=f.sql("SELECT job_id FROM crm.coordination_runs WHERE deal_id='"+deal['id']+"'")
 before=counts();remove_key=key()
 f.sql('REVOKE UPDATE ON crm.starter_installations FROM looski_runtime')
 try:remove(first,remove_key,503)
 finally:f.sql('GRANT UPDATE ON crm.starter_installations TO looski_runtime')
 assert counts()==before and call(path+'/'+first['id'])==first
 removed=remove(first,remove_key)
 assert removed['removed'] and len(removed['removal'])==16 and all(d['outcome']=='archived' for d in removed['removal'])
 assert all(r['archived'] for r in removed['records']) and remove(first,remove_key)==removed
 assert call(base+'/crm/opportunities/'+opp['id'])['id']==opp['id']
 assert call(base+'/crm/deals/'+deal['id'])['originating_opportunity_id']==opp['id']
 assert call(base+'/crm/activities/'+activity['id'])==activity
 assert call(base+'/tasks/'+task['id'])==task
 assert call(base+'/crm/deals/'+deal['id']+'/coordination/'+instance['id'])==instance
 for attempt in range(50):
  if call(base+'/jobs/'+job)['state']=='blocked':break
  # Compose exec bypasses the entrypoint's secret-path exports. Linux bind
  # mounts retain the host UID, so every required key must use its runtime copy.
  subprocess.run(['docker','compose','exec','-T','--user','10001',
                  '-e','LOOSKI_DATABASE_URL_FILE=/run/looski/database_url',
                  '-e','LOOSKI_JWT_SECRET_FILE=/run/looski/jwt_secret',
                  '-e','LOOSKI_INVITATION_KEY_FILE=/run/looski/invitation_key',
                  'api','looski','worker','--once'],cwd=ROOT,env=f.environment,check=True,stdout=subprocess.DEVNULL)
 assert call(base+'/jobs/'+job)['state']=='blocked'
finally:subprocess.run(['docker','compose','start','worker'],cwd=ROOT,env=f.environment,check=True,capture_output=True)
print('PASS: removal rollback, archive-only retention of opportunities/deals/activities/tasks/checklists and blocked pending work')
# Concurrent installs, with both same-key and competing-key clients.
for shared in [True,False]:
 retry=key();before=int(f.sql('SELECT count(*) FROM crm.starter_installations'))
 def race(n):
  req=Request(f.base+path,method='POST',headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':retry if shared else key()},data=b'{"package_version":1}')
  try:r=urlopen(req,timeout=20)
  except HTTPError as e:r=e
  with r:return r.status,json.loads(r.read())
 with ThreadPoolExecutor(2) as pool:results=list(pool.map(race,[1,2]))
 assert sorted(r[0] for r in results)==([201,201] if shared else [201,409]),results
 if shared:assert results[0][1]==results[1][1]
 second=next(r[1] for r in results if r[0]==201)
 assert int(f.sql('SELECT count(*) FROM crm.starter_installations'))==before+1
 assert not set(r['id'] for r in first['records']) & set(r['id'] for r in second['records'])
 remove(second)
# Customized rule keeps its status and checklist, which keeps Tasks definitions.
custom=install();rule=call(config_path(record(custom,'coordination-rules')))
call(config_path(record(custom,'coordination-rules')),'PATCH',{'name':'Customized '+name,'status_id':rule['status_id'],'revision_id':rule['revision_id'],'expected_version':rule['version']},key=key())
new_stage=create('/crm/stages',{'name':'Custom stage','parent_id':rid(custom,'pipelines'),'outcome':'open'})
retired=remove(custom);decisions={d['id']:d for d in retired['removal']}
for kind in ['coordination-rules','coordination-templates','task-templates','task-types','categories','pipelines']:
 assert decisions[rid(custom,kind)]['outcome']=='retained',(kind,decisions)
assert decisions[rid(custom,'statuses','Active')]['outcome']=='retained'
assert decisions[rid(custom,'activity-types','Note')]['outcome']=='archived'
assert call(base+'/crm/stages/'+new_stage['id'])['archived'] is False
# Earlier revisions of an active head remain explicitly usable.
legacy=install();old_coord=call(config_path(record(legacy,'coordination-templates')))
alternative=create('/tasks/templates',{'name':name+' alternative','title':'Alternative work','assignment':'initiator','due_offset_days':0})
call(config_path(record(legacy,'coordination-templates')),'PATCH',{'name':'Changed checklist','expected_version':1,'items':[{'label':'Alternative','task_template_revision_id':alternative['revision']['id']}]},key=key())
result=remove(legacy);outcomes={d['id']:d['outcome'] for d in result['removal']}
assert outcomes[rid(legacy,'task-templates')]=='retained' and outcomes[rid(legacy,'task-types')]=='retained'
legacy=install();template=call(config_path(record(legacy,'task-templates')))
call(config_path(record(legacy,'task-templates')),'PATCH',{'name':'Changed task','title':'Changed work','assignment':'initiator','due_offset_days':0,'type_id':None,'expected_version':1},key=key())
result=remove(legacy);assert next(d for d in result['removal'] if d['id']==rid(legacy,'task-types'))['outcome']=='retained'
print('PASS: concurrent installs, fresh definition identities on reinstall and transitive retention for customized configuration')
# Current access is enforced on both fresh operations and cached results.
role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
for permission in ['crm.configuration.manage','crm.configuration.read','tasks.configuration.manage','tasks.configuration.read']:
 f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='"+permission+"'")
 try:
  call(path,'POST',{'package_version':1},expected=403,key=failed)
  remove(first,remove_key,403)
  if permission.endswith('.read'):call(path,expected=403)
 finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','"+permission+"')")
module=next(m for m in call(base+'/modules') if m['key']=='crm')
call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key())
try:call(path,expected=403);call(path,'POST',{'package_version':1},expected=403,key=failed)
finally:call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']+1},key=key())
assert not f.sql("SELECT id FROM crm.starter_installations WHERE workspace_id='"+f.workspace+"' AND removed_at IS NULL")
assert call(path+'?limit=1')['next_cursor'] is not None
other=f.sql("INSERT INTO suite.workspaces(name) VALUES('Starter foreign') RETURNING id").splitlines()[0]
f.sql("DO $$ BEGIN BEGIN UPDATE crm.starter_installations SET workspace_id='"+other+"' WHERE id='"+first['id']+"'; RAISE EXCEPTION 'Foreign references accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
call('/api/v1/workspaces/'+other+'/crm/starter-packages/'+first['id'],expected=403)
assert f.cli(['crm','starter-package',first['id']])['id']==first['id']
assert not f.mcp('crm_starter_package',{'workspace_id':f.workspace,'id':first['id']}).get('isError')
print('PASS: current grant/module checks including replay, scoped typed references, no implicit reinstall, CLI/MCP parity')
# Mutation adapters must preserve keys, JSON inputs, versions and decisions.
cli_key=key();cli_args=['crm','install-starter','--idempotency-key',cli_key]
cli_item=f.cli(cli_args,{'package_version':1});assert f.cli(cli_args,{'package_version':1})==cli_item
cli_remove=['crm','remove-starter',cli_item['id'],'--idempotency-key',key()]
cli_result=f.cli(cli_remove,{'expected_version':cli_item['version']});assert cli_result['removed'] and len(cli_result['removal'])==16
assert f.cli(cli_remove,{'expected_version':cli_item['version']})==cli_result
def mcp_data(tool,args):
 r=f.mcp(tool,args);assert not r.get('isError'),r
 return json.loads(r['content'][0]['text'])
args={'workspace_id':f.workspace,'idempotency_key':key(),'change':{'package_version':1}}
mcp_item=mcp_data('crm_install_starter',args);assert mcp_data('crm_install_starter',args)==mcp_item
args={'workspace_id':f.workspace,'id':mcp_item['id'],'idempotency_key':key(),'change':{'expected_version':999}}
assert f.mcp('crm_remove_starter',args).get('isError')
args['change']['expected_version']=mcp_item['version'];mcp_result=mcp_data('crm_remove_starter',args)
assert mcp_result['removed'] and len(mcp_result['removal'])==16 and mcp_data('crm_remove_starter',args)==mcp_result
print('PASS: CLI/MCP installation, removal, JSON input, retry identity, stale versions and returned decisions')
# Browser install/removal preserves invalid and stale form values.
def browser(route,values=None,expected=200):
 req=Request(f.base+route,data=None if values is None else urlencode(values).encode(),headers={'Origin':f.base,'Accept':'text/html'})
 try:r=f.browser.open(req,timeout=20)
 except HTTPError as e:r=e
 with r:
  html=r.read().decode();assert r.status==expected,(route,r.status,html[:250]);return Forms(html),html,r.url
web='/workspaces/'+f.workspace+'/crm/starter-packages'
forms,html,_=browser(web);assert 'Removal archives unchanged defaults' in html
form=forms.form('package_version');form['package_version']='bad';forms,_,_=browser(web,form,400);assert forms.form('package_version')==form
form['package_version']='1';forms,_,url=browser(web,form);route=urlparse(url).path
stale=forms.form('expected_version');stale['expected_version']='999';forms,html,_=browser(route,stale,409);assert forms.form('expected_version')==stale and 'Discard draft' in html
forms,_,_=browser(route);_,html,_=browser(route,forms.form('expected_version'));assert 'Removal results' in html and 'Remove starter package</button>' not in html
print('PASS: SSR installation/removal, rejected draft retention and visible removal decisions')

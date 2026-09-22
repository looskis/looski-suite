#!/usr/bin/env python3
"""Durable deal-status coordination with current authority and bounded retries."""
from integration_fixture import Fixture
f=Fixture();base='/api/v1/workspaces/'+f.workspace
assert f.call(base+'/jobs')['items'] is not None
print('PASS: authorized workspace job inspection')
import secrets,subprocess,time,json
from concurrent.futures import ThreadPoolExecutor
from dev import ROOT
call=f.call
name='Status work '+secrets.token_hex(5)
def key():return secrets.token_hex(16)
def create(path,body):return call(base+path,'POST',body,expected=201,key=key())
def compose(*args,**kwargs):return subprocess.run(['docker','compose',*args],cwd=ROOT,env=f.environment,check=True,**kwargs)
# Run deterministic worker commands in the API container while the daemon is stopped.
def once():
 command=['docker','compose','exec','-T','--user','10001','-e','LOOSKI_DATABASE_URL_FILE=/run/looski/database_url','-e','LOOSKI_JWT_SECRET_FILE=/run/looski/jwt_secret','-e','LOOSKI_INVITATION_KEY_FILE=/run/looski/invitation_key','api','looski','worker','--once']
 return subprocess.run(command,cwd=ROOT,env=f.environment,check=True,capture_output=True,text=True)
def queued():return f.sql("SELECT id FROM suite.jobs WHERE workspace_id='"+f.workspace+"' ORDER BY created_at DESC,id DESC LIMIT 1")
def wake(job):f.sql("SELECT pgmq.set_vt('looski_jobs',message_id,0) FROM suite.jobs WHERE id='"+job+"'")
def job_state(job):return call(base+'/jobs/'+job)
def change_job(job,action,expected=200):
 current=job_state(job);return call(base+'/jobs/'+job,'PATCH',{'expected_version':current['version'],'action':action},expected=expected,key=key())
module=next(m for m in call(base+'/modules') if m['key']=='crm')
if not module['enabled']:call(base+'/modules/crm','PATCH',{'expected_version':module['version'],'enabled':True},key=key())
compose('stop','worker',stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
try:
 work=create('/tasks/templates',{'name':name,'title':name+' task','assignment':'initiator','due_offset_days':1})
 template=create('/crm/coordination-templates',{'name':name,'items':[{'label':'Follow up','task_template_revision_id':work['revision']['id']}]})
 category=create('/crm/categories',{'name':name});status_a=create('/crm/statuses',{'name':name+' initial','parent_id':category['id']});status_b=create('/crm/statuses',{'name':name+' agreed','parent_id':category['id']})
 rule_input={'name':name,'status_id':status_b['id'],'revision_id':template['revision']['id']}
 rule=create('/crm/coordination-rules',rule_input)
 call(base+'/crm/coordination-rules','POST',rule_input,expected=409,key=key())
 def new_deal(status=None):return create('/crm/deals',{'title':name,'category_id':category['id'],'status_id':status or status_a['id']})
 def transition(deal,status=None,k=None,expected=200):
  current=call(base+'/crm/deals/'+deal['id']);payload={'title':current['title'],'category_id':category['id'],'status_id':status or status_b['id'],'expected_version':current['version']}
  return call(base+'/crm/deals/'+deal['id'],'PATCH',payload,key=k or key(),expected=expected)
 before=f.sql('SELECT count(*) FROM suite.jobs');direct=new_deal(status_b['id']);assert f.sql('SELECT count(*) FROM suite.jobs')==before
 deal=new_deal();current=call(base+'/crm/deals/'+deal['id']);payload={'title':name,'category_id':category['id'],'status_id':status_b['id'],'expected_version':current['version']};retry=key()
 result=call(base+'/crm/deals/'+deal['id'],'PATCH',payload,key=retry);job=queued()
 assert call(base+'/crm/deals/'+deal['id'],'PATCH',payload,key=retry)==result
 assert job_state(job)['state']=='queued' and job_state(job)['context']['deal_id']==deal['id']
 transition(deal);assert queued()==job
 call(base+'/crm/statuses/'+status_b['id'],'PATCH',{'name':name+' renamed','parent_id':category['id'],'expected_version':1},key=key());assert queued()==job
 # Queue insertion and deal transition share the business transaction.
 rollback_deal=new_deal();before=f.sql('SELECT count(*) FROM suite.jobs');before_queue=f.sql('SELECT count(*) FROM pgmq.q_looski_jobs')
 f.sql('REVOKE INSERT ON crm.coordination_runs FROM looski_runtime')
 try:transition(rollback_deal,expected=503)
 finally:f.sql('GRANT INSERT ON crm.coordination_runs TO looski_runtime')
 assert call(base+'/crm/deals/'+rollback_deal['id'])==rollback_deal
 assert f.sql('SELECT count(*) FROM suite.jobs')==before and f.sql('SELECT count(*) FROM pgmq.q_looski_jobs')==before_queue
 # Rules are pinned at scheduling, so later edits cannot change already queued work.
 new_template=create('/crm/coordination-templates',{'name':name+' next','items':[{'label':'Future work','task_template_revision_id':work['revision']['id']}]})
 rule=call(base+'/crm/coordination-rules/'+rule['id'],'PATCH',{**rule_input,'revision_id':new_template['revision']['id'],'expected_version':rule['version']},key=key())
 once();completed=job_state(job);assert completed['state']=='completed' and completed['attempts']==1
 instance=call(base+'/crm/deals/'+deal['id']+'/coordination/'+completed['context']['instance_id'])
 assert instance['revision_id']==template['revision']['id'] and instance['items'][0]['label']=='Follow up'
 task=instance['items'][0]['task'];assert task['source_template_revision_id']==work['revision']['id']
 assert [e['state'] for e in call(base+'/jobs/'+job+'/history')['items']]==['queued','running','completed']
 count=f.sql('SELECT count(*) FROM tasks.tasks')
 f.sql("SELECT pgmq.send('looski_jobs',jsonb_build_object('job_id','"+job+"'))")
 once();assert f.sql('SELECT count(*) FROM tasks.tasks')==count
 print('PASS: material transition-only scheduling, atomic PGMQ rollback, pinned rule execution and durable duplicate delivery')
 # Revocation never promotes the operator who resumes work to initiating authority.
 role=f.sql("SELECT id FROM suite.roles WHERE workspace_id='"+f.workspace+"' AND key='owner'")
 denied=new_deal();transition(denied);blocked=queued()
 f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='tasks.create'")
 try:
  once();assert job_state(blocked)['state']=='blocked'
  change_job(blocked,'resume');once();assert job_state(blocked)['state']=='blocked'
 finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','tasks.create')")
 change_job(blocked,'resume');once();assert job_state(blocked)['state']=='completed'
 # Disabling before claim blocks notifications immediately and re-enable never resumes.
 disabled=new_deal();transition(disabled);blocked=queued();module=next(m for m in call(base+'/modules') if m['key']=='crm')
 module=call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key())
 assert job_state(blocked)['state']=='blocked' and job_state(blocked)['context'] is None
 assert job_state(blocked)['error_code']=='module_disabled';change_job(blocked,'resume',403)
 module=call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key())
 once();assert job_state(blocked)['state']=='blocked'
 resume_input={'expected_version':job_state(blocked)['version'],'action':'resume'};resume_key=key()
 call(base+'/jobs/'+blocked,'PATCH',resume_input,key=resume_key)
 module=call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key())
 call(base+'/jobs/'+blocked,'PATCH',resume_input,key=resume_key,expected=403)
 module=call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key())
 change_job(blocked,'resume');once();assert job_state(blocked)['state']=='completed'
 # Moving away and back creates a distinct transition; stale original work blocks.
 moved=new_deal();transition(moved);obsolete=queued();transition(moved,status_a['id']);transition(moved);latest=queued()
 once();assert job_state(obsolete)['state']=='blocked';once();assert job_state(latest)['state']=='completed'
 print('PASS: current initiating authority, explicit resume, module blocking without implicit restart and obsolete-transition protection')
 # Retirement of a referenced status blocks already scheduled work.
 retired=new_deal();transition(retired);retired_job=queued();definition=call(base+'/crm/statuses/'+status_b['id'])
 definition=call(base+'/crm/statuses/'+status_b['id'],'PATCH',{'name':definition['name'],'parent_id':category['id'],'expected_version':definition['version'],'archived':True},key=key())
 once();assert job_state(retired_job)['state']=='blocked'
 call(base+'/crm/statuses/'+status_b['id'],'PATCH',{'name':definition['name'],'parent_id':category['id'],'expected_version':definition['version'],'archived':False},key=key())
 change_job(retired_job,'resume');once();assert job_state(retired_job)['state']=='completed'

 # A database failure rolls back every effect, retries with a bounded budget, then exhausts.
 failing=new_deal();transition(failing);failed=queued();before=f.sql('SELECT count(*) FROM tasks.tasks')
 f.sql('REVOKE INSERT ON crm.deal_coordination_items FROM looski_runtime')
 try:
  for attempt in range(1,6):
   wake(failed);once();current=job_state(failed);assert current['attempts']==attempt and current['state']==('exhausted' if attempt==5 else 'queued'),current
   assert f.sql('SELECT count(*) FROM tasks.tasks')==before
 finally:f.sql('GRANT INSERT ON crm.deal_coordination_items TO looski_runtime')
 change_job(failed,'resume');once();assert job_state(failed)['state']=='completed'
 assert len(call(base+'/jobs/'+failed+'/history')['items'])==14
 cancelled=new_deal();transition(cancelled);cancelled_job=queued();change_job(cancelled_job,'cancel');once();assert job_state(cancelled_job)['state']=='cancelled'
 change_job(cancelled_job,'resume',409)
 print('PASS: transactional execution rollback, bounded delayed retry/exhaustion, explicit new retry run and cancellation')
 # Fence replacement while the first worker is waiting for the workspace lock.
 raced=new_deal();transition(raced);raced_job=queued()
 lock_sql="SET application_name='looski-job-fence-fixture'; BEGIN; SELECT id FROM suite.workspaces WHERE id='"+f.workspace+"' FOR NO KEY UPDATE; SELECT pg_sleep(30); COMMIT;"
 locker=subprocess.Popen(['docker','compose','exec','-T','db','psql','-U','postgres','-d','postgres','-At','-v','ON_ERROR_STOP=1'],stdin=subprocess.PIPE,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,text=True,cwd=ROOT,env=f.environment)
 locker.stdin.write(lock_sql);locker.stdin.close()
 def wait_for(predicate):
  deadline=time.monotonic()+8
  while not predicate():
   assert time.monotonic()<deadline,'Timed out waiting for isolated worker state';time.sleep(.05)
 worker_command=['docker','compose','exec','-T','--user','10001','-e','LOOSKI_DATABASE_URL_FILE=/run/looski/database_url','-e','LOOSKI_JWT_SECRET_FILE=/run/looski/jwt_secret','-e','LOOSKI_INVITATION_KEY_FILE=/run/looski/invitation_key','api','looski','worker','--once']
 workers=[]
 try:
  wait_for(lambda:f.sql("SELECT count(*) FROM pg_stat_activity WHERE application_name='looski-job-fence-fixture' AND wait_event='PgSleep'")=='1')
  workers.append(subprocess.Popen(worker_command,cwd=ROOT,env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True))
  wait_for(lambda:f.sql("SELECT attempts FROM suite.jobs WHERE id='"+raced_job+"'")=='1')
  old=f.sql("SELECT lease_token FROM suite.jobs WHERE id='"+raced_job+"'")
  f.sql("UPDATE suite.jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE id='"+raced_job+"'");wake(raced_job)
  workers.append(subprocess.Popen(worker_command,cwd=ROOT,env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True))
  wait_for(lambda:f.sql("SELECT attempts FROM suite.jobs WHERE id='"+raced_job+"'")=='2')
  assert f.sql("SELECT lease_token FROM suite.jobs WHERE id='"+raced_job+"'")!=old
 finally:
  f.sql("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE application_name='looski-job-fence-fixture'")
  locker.wait(timeout=10)
 for worker in workers:
  out,err=worker.communicate(timeout=20);assert worker.returncode==0,(out,err)
 assert job_state(raced_job)['state']=='completed' and len(call(base+'/crm/deals/'+raced['id']+'/coordination')['items'])==1
 print('PASS: expired lease reclamation fences the old worker and commits exactly one checklist')
 # Actual process death preserves a claimed job and rolls back staged business writes.
 for staged in [False,True]:
  crashed=new_deal();transition(crashed);crash_job=queued();before=f.sql('SELECT count(*) FROM tasks.tasks')
  marker='looski-job-crash-'+secrets.token_hex(4)
  lock_process=None
  if staged:
   f.sql("CREATE FUNCTION public."+marker.replace('-','_')+"() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_sleep(30); RETURN NEW; END $$; CREATE TRIGGER "+marker.replace('-','_')+" AFTER INSERT ON crm.deal_coordination_items FOR EACH ROW WHEN (NEW.deal_id='"+crashed['id']+"') EXECUTE FUNCTION public."+marker.replace('-','_')+"();")
  else:
   lock_process=subprocess.Popen(['docker','compose','exec','-T','db','psql','-U','postgres','-d','postgres','-At','-v','ON_ERROR_STOP=1'],stdin=subprocess.PIPE,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,text=True,cwd=ROOT,env=f.environment)
   lock_process.stdin.write("SET application_name='"+marker+"'; BEGIN; SELECT id FROM suite.workspaces WHERE id='"+f.workspace+"' FOR NO KEY UPDATE; SELECT pg_sleep(30); COMMIT;");lock_process.stdin.close()
   wait_for(lambda:f.sql("SELECT count(*) FROM pg_stat_activity WHERE application_name='"+marker+"' AND wait_event='PgSleep'")=='1')
  crash_command=['docker','compose','exec','-T','--user','10001','-e','LOOSKI_DATABASE_URL_FILE=/run/looski/database_url','-e','LOOSKI_JWT_SECRET_FILE=/run/looski/jwt_secret','-e','LOOSKI_INVITATION_KEY_FILE=/run/looski/invitation_key','api','sh','-c','echo $$ > /tmp/'+marker+'.pid; exec looski worker --once']
  process=subprocess.Popen(crash_command,cwd=ROOT,env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  try:
   wait_for(lambda:f.sql("SELECT attempts FROM suite.jobs WHERE id='"+crash_job+"'")=='1')
   if staged:wait_for(lambda:f.sql("SELECT count(*) FROM pg_stat_activity WHERE query LIKE 'INSERT INTO crm.deal_coordination_items%' AND wait_event='PgSleep'")=='1')
   compose('exec','-T','--user','10001','api','sh','-c','kill -9 "$(cat /tmp/'+marker+'.pid)"',stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
   process.communicate(timeout=10);assert process.returncode!=0
   if staged:
    # The sleeping database statement may notice the lost socket only later.
    f.sql("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE query LIKE 'INSERT INTO crm.deal_coordination_items%' AND wait_event='PgSleep'")
  finally:
   if staged:f.sql('DROP TRIGGER '+marker.replace('-','_')+' ON crm.deal_coordination_items; DROP FUNCTION public.'+marker.replace('-','_')+'();')
   else:
    f.sql("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE application_name='"+marker+"'");lock_process.wait(timeout=10)
  assert f.sql('SELECT count(*) FROM tasks.tasks')==before
  assert job_state(crash_job)['state']=='running'
  f.sql("UPDATE suite.jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE id='"+crash_job+"'");wake(crash_job);once()
  assert job_state(crash_job)['state']=='completed' and len(call(base+'/crm/deals/'+crashed['id']+'/coordination')['items'])==1
 print('PASS: actual worker death after claim and after staged writes recovers with one committed checklist and no orphan tasks')
 # Disable starts before claim but appends its blocked event after that claim.
 ordered=new_deal();transition(ordered);ordered_job=queued()
 locker=subprocess.Popen(['docker','compose','exec','-T','db','psql','-U','postgres','-d','postgres','-At','-v','ON_ERROR_STOP=1'],stdin=subprocess.PIPE,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,text=True,cwd=ROOT,env=f.environment)
 locker.stdin.write("SET application_name='looski-job-order-fixture'; BEGIN; SELECT id FROM suite.workspaces WHERE id='"+f.workspace+"' FOR NO KEY UPDATE; SELECT pg_sleep(30); COMMIT;");locker.stdin.close()
 wait_for(lambda:f.sql("SELECT count(*) FROM pg_stat_activity WHERE application_name='looski-job-order-fixture' AND wait_event='PgSleep'")=='1')
 module=next(m for m in call(base+'/modules') if m['key']=='crm')
 with ThreadPoolExecutor(1) as executor:
  pending=executor.submit(lambda:call(base+'/modules/crm','PATCH',{'enabled':False,'expected_version':module['version']},key=key()))
  worker=None
  try:
   wait_for(lambda:int(f.sql("SELECT count(*) FROM pg_stat_activity WHERE query LIKE 'SELECT id FROM suite.workspaces%' AND wait_event_type='Lock'"))>=1)
   worker=subprocess.Popen(worker_command,cwd=ROOT,env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
   wait_for(lambda:f.sql("SELECT attempts FROM suite.jobs WHERE id='"+ordered_job+"'")=='1')
  finally:
   f.sql("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE application_name='looski-job-order-fixture'");locker.wait(timeout=10)
  module=pending.result(timeout=15)
  if worker:
   out,err=worker.communicate(timeout=15);assert worker.returncode==0,(out,err)
 assert [e['state'] for e in call(base+'/jobs/'+ordered_job+'/history')['items']]==['queued','running','blocked']
 call(base+'/modules/crm','PATCH',{'enabled':True,'expected_version':module['version']},key=key());change_job(ordered_job,'cancel')
 print('PASS: attempt history follows serialized transitions even when the disabling transaction starts earlier')

 # Job metadata is safe while business context is separately filtered.
 other=f.sql("WITH u AS (INSERT INTO suite.users(display_name) VALUES('Job scope owner') RETURNING id) INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+f.workspace+"',id FROM u RETURNING id").splitlines()[0]
 d=call(base+'/crm/deals/'+deal['id']);call(base+'/crm/deals/'+deal['id'],'PATCH',{'title':d['title'],'category_id':d['category_id'],'status_id':d['status_id'],'owner_membership_id':other,'expected_version':d['version']},key=key())
 f.sql("UPDATE suite.role_permissions SET record_scope='assigned' WHERE role_id='"+role+"' AND permission_key='crm.deals.read'")
 try:assert job_state(job)['context'] is None
 finally:f.sql("UPDATE suite.role_permissions SET record_scope='workspace' WHERE role_id='"+role+"' AND permission_key='crm.deals.read'")
 f.sql("DELETE FROM suite.role_permissions WHERE role_id='"+role+"' AND permission_key='workspace.jobs.read'")
 try:call(base+'/jobs',expected=403);call(base+'/jobs/'+job,expected=403)
 finally:f.sql("INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) VALUES('"+f.workspace+"','"+role+"','workspace.jobs.read')")
 assert f.cli(['jobs','get',job])['state']=='completed'
 assert not f.mcp('jobs_history',{'workspace_id':f.workspace,'id':job}).get('isError')
 print('PASS: separately scoped job context, operator permission denial and CLI/MCP job inspection')
 # Identity revocation is checked from the durable actor, without retaining a bearer token.
 revoked=new_deal();transition(revoked);revoked_job=queued()
 identity=f.sql("SELECT i.id FROM suite.auth_identities i JOIN suite.jobs j ON j.actor_user_id=i.user_id AND j.actor_issuer=i.issuer AND j.actor_subject=i.subject WHERE j.id='"+revoked_job+"'")
 f.sql("UPDATE suite.auth_identities SET disabled_at=now() WHERE id='"+identity+"'")
 try:
  once();assert f.sql("SELECT state FROM suite.jobs WHERE id='"+revoked_job+"'")=='blocked'
 finally:f.sql("UPDATE suite.auth_identities SET disabled_at=NULL WHERE id='"+identity+"'")
 change_job(revoked_job,'resume');once();assert job_state(revoked_job)['state']=='completed'
 # Browser rule configuration and operator actions preserve invalid/conflicting drafts.
 from browser_forms import Forms
 from urllib.request import Request
 from urllib.error import HTTPError
 from urllib.parse import urlencode,urlparse
 def browser(route,values=None,expected=200):
  req=Request(f.base+route,data=None if values is None else urlencode(values).encode(),headers={'Origin':f.base,'Accept':'text/html'})
  try:r=f.browser.open(req,timeout=20)
  except HTTPError as e:r=e
  with r:
   html=r.read().decode();assert r.status==expected,(route,r.status,html[:250]);return Forms(html),html,r.url
 web='/workspaces/'+f.workspace;listing=web+'/crm/coordination-rules'
 forms,_,_=browser(listing);draft=forms.form('name');draft.update(name=name+' browser',status_id='invalid',revision_id=template['revision']['id'])
 forms,_,_=browser(listing,draft,400);assert forms.form('name')==draft
 third=create('/crm/statuses',{'name':name+' browser','parent_id':category['id']});draft['status_id']=third['id']
 forms,_,url=browser(listing,draft);rule_id=urlparse(url).path.rsplit('/',1)[1];stale=forms.form('name');current=call(base+'/crm/coordination-rules/'+rule_id)
 call(base+'/crm/coordination-rules/'+rule_id,'PATCH',{'name':current['name']+' server','status_id':current['status_id'],'revision_id':current['revision_id'],'expected_version':current['version']},key=key())
 forms,html,_=browser(listing+'/'+rule_id,stale,409);assert forms.form('name')==stale and 'Discard draft' in html
 browser_deal=new_deal();transition(browser_deal,third['id']);browser_job=queued();job_page=web+'/jobs/'+browser_job
 forms,html,_=browser(job_page);assert browser_deal['title'] in html;draft=forms.form('action');draft['action']='invalid'
 forms,_,_=browser(job_page,draft,400);assert forms.form('action')==draft
 draft['action']='cancel';change_job(browser_job,'cancel');forms,html,_=browser(job_page,draft,409);assert forms.form('action')==draft and 'Discard draft' in html
 _,html,_=browser(web+'/jobs/'+job);assert 'View created checklist' in html
 page=call(base+'/jobs?limit=1');assert page['next_cursor'] and call(base+'/jobs?limit=1&after='+page['next_cursor'])['items']
 history=call(base+'/jobs/'+failed+'/history?limit=2');assert history['next_cursor'] and call(base+'/jobs/'+failed+'/history?after='+history['next_cursor'])['items']
 call(base+'/jobs/'+failed+'/history?after='+job,expected=404)
 print('PASS: revoked identity blocks queued authority, SSR rule/job drafts, visible outcome links and bounded history pagination')
 # Real foreign-workspace rows cannot be inspected, resumed or cross-referenced.
 foreign=f.sql("INSERT INTO suite.workspaces(name) VALUES('Foreign job workspace') RETURNING id").splitlines()[0]
 foreign_member=f.sql("INSERT INTO suite.memberships(workspace_id,user_id) SELECT '"+foreign+"',actor_user_id FROM suite.jobs WHERE id='"+job+"' RETURNING id").splitlines()[0]
 foreign_job=f.sql("INSERT INTO suite.jobs(workspace_id,kind,deduplication_key,actor_user_id,actor_membership_id,actor_issuer,actor_subject,state) SELECT '"+foreign+"','crm.coordination','foreign',actor_user_id,'"+foreign_member+"',actor_issuer,actor_subject,'blocked' FROM suite.jobs WHERE id='"+job+"' RETURNING id").splitlines()[0]
 call(base+'/jobs/'+foreign_job,expected=404);call(base+'/jobs/'+foreign_job+'/history',expected=404)
 call(base+'/jobs/'+foreign_job,'PATCH',{'expected_version':1,'action':'cancel'},expected=404,key=key())
 call('/api/v1/workspaces/'+foreign+'/jobs',expected=403)
 f.sql("DO $$ BEGIN BEGIN UPDATE crm.coordination_runs SET job_id='"+foreign_job+"' WHERE job_id='"+job+"'; RAISE EXCEPTION 'Foreign job accepted'; EXCEPTION WHEN foreign_key_violation THEN NULL; END; END $$;")
 print('PASS: foreign workspace job denial and typed workspace references')


finally:
 compose('up','-d','--no-deps','--wait','worker',stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

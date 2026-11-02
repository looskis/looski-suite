#!/usr/bin/env python3
"""Atomic booking/intake, calendar conflicts, DST, durable follow-up and reminder fencing."""
import datetime,json,secrets,subprocess,time,uuid
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request,urlopen
from urllib.error import HTTPError
from integration_fixture import Fixture
from fixture_people import people
from dev import ROOT
f=Fixture();call,sql=f.call,f.sql;base='/api/v1/workspaces/'+f.workspace;root=base+'/scheduling';public='/api/v1/public/scheduling';key=lambda:secrets.token_hex(16)
def module(k,enabled):
 m=next(x for x in call(base+'/modules') if x['key']==k)
 if m['enabled']!=enabled:call(base+'/modules/'+k,'PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
def compose(*args):
 if args==('start','worker'):args=('up','-d','--no-deps','worker')
 subprocess.run(['docker','compose',*args],cwd=ROOT,env=f.environment,check=True,stdout=subprocess.DEVNULL)
def make_type(**changes):return call(root+'/types','POST',{'settings':{**settings,**changes}},key=key())
def revise(t,**changes):return call(root+'/types/'+t['id'],'PUT',{'expected_version':t['version'],'settings':{**t['settings'],**changes}},key=key())
def pub(path,method='GET',body=None,expected=200,code=None,retry=None):return call(public+path,method,body,expected=expected,authenticated=False,headers={'X-Booking-Key':code or secrets.token_hex(32),'X-Forwarded-For':'203.0.113.37'},key=retry)
def book(t,start,code=None,**changes):
 code=code or secrets.token_hex(32);payload={'starts_at':start,'attendee_name':'Attendee','attendee_email':'booking-'+key()+'@example.invalid',**changes};return pub('/types/'+t['public_key']+'/book','POST',payload,code=code),code,payload
def get(id):return call(root+'/appointments/'+id)
def wait(id,state='completed'):
 for _ in range(60):
  b=get(id)
  if b['job_state']==state:return b
  time.sleep(.25)
 raise AssertionError((b['recipe_state'],b['job_state'],b['error_code']))
def customer(id,code,action=None,expected=200,**changes):
 if action is None:return pub('/appointments/'+id,code=code,expected=expected)
 b=customer(id,code);return pub('/appointments/'+id,'POST',{'expected_version':b['version'],'action':action,**changes},expected=expected,code=code,retry=key())
day=(datetime.datetime.now(datetime.timezone.utc)+datetime.timedelta(days=2)).date()
while int(sql(f"SELECT count(*) FROM scheduling.appointments WHERE workspace_id='{f.workspace}' AND starts_at::date='{day}'")):day+=datetime.timedelta(days=1)
day=day.isoformat();at=lambda h:f'{day}T{h}:00Z'
settings={'name':'Appointment '+key(),'description':'Test booking','timezone':'UTC','duration_minutes':30,'buffer_before':15,'buffer_after':15,'weekly':[{'weekday':i,'start':0,'end':1440} for i in range(1,8)],'accepting':True,'archived':False,'reminder_minutes':0}
initial={m['key']:m['enabled'] for m in call(base+'/modules')}
try:
 for m in initial:module(m,m=='scheduling')
 t=make_type();t2=make_type(name='Second type '+key())
 info=pub('/types/'+t['public_key']);assert info['form'] is None and 'crm' not in info and 'host_id' not in info
 slots=pub('/types/'+t['public_key']+'/slots?date='+day);assert slots and at('12:00') in [s['starts_at'] for s in slots]
 def compete(t):
  code=secrets.token_hex(32);payload={'starts_at':at('12:00'),'attendee_name':'Concurrent','attendee_email':'race@example.invalid'}
  request=Request(f.base+public+'/types/'+t['public_key']+'/book',data=json.dumps(payload).encode(),method='POST',headers={'Content-Type':'application/json','X-Booking-Key':code})
  try:r=urlopen(request)
  except HTTPError as e:r=e
  with r:return r.status,json.loads(r.read()),code
 with ThreadPoolExecutor(2) as pool:race=list(pool.map(compete,[t,t2]))
 assert sorted(r[0] for r in race)==[200,409];won=next(r for r in race if r[0]==200);id,code=won[1]['id'],won[2]
 # Buffers cover adjacent starts across different appointment types.
 pub('/types/'+t['public_key']+'/book','POST',{'starts_at':at('12:30'),'attendee_name':'Buffer','attendee_email':'buffer@example.invalid'},expected=409)
 customer(id,secrets.token_hex(32),expected=404)
 assert 'attendee_email' not in customer(id,code)
 customer(id,code,'reschedule',starts_at=at('14:00'));assert get(id)['starts_at']==at('14:00')
 customer(id,code,'cancel');assert get(id)['state']=='cancelled'
 assert len(call(root+'/appointments/'+id+'/events')['items'])==3
 standalone,sc,payload=book(t,at('12:00'));sid=standalone['id']
 assert pub('/types/'+t['public_key']+'/book','POST',payload,code=sc)['id']==sid
 pub('/types/'+t['public_key']+'/book','POST',{**payload,'attendee_name':'Different'},code=sc,expected=409)
 assert get(sid)['recipe_state']=='none'
 # Date exceptions replace weekly hours; editing availability preserves existing reservations.
 t=revise(t,exceptions=[{'date':day,'windows':[]}]);assert pub('/types/'+t['public_key']+'/slots?date='+day)==[];assert customer(sid,sc)['state']=='confirmed';t=revise(t,exceptions=[])
 # Actual UTC enumeration exercises spring gaps and both fall-fold occurrences.
 from zoneinfo import ZoneInfo
 transitions={}
 today=datetime.datetime.now(datetime.timezone.utc).date()
 for zone in ['Europe/London','Australia/Sydney']:
  for days in range(1,366):
   date=today+datetime.timedelta(days=days)
   before=datetime.datetime.combine(date,datetime.time(0),tzinfo=ZoneInfo(zone)).utcoffset()
   after=datetime.datetime.combine(date,datetime.time(4),tzinfo=ZoneInfo(zone)).utcoffset()
   if before!=after:
    kind='gap' if after>before else 'fold'
    transitions.setdefault(kind,(zone,date))
 assert set(transitions)=={'gap','fold'}
 for kind,(zone,date) in transitions.items():
  dst=make_type(timezone=zone,duration_minutes=15,buffer_before=0,buffer_after=0,weekly=[{'weekday':date.isoweekday(),'start':0,'end':240}])
  local=pub('/types/'+dst['public_key']+'/slots?date='+date.isoformat())
  assert len(local)==(12 if kind=='gap' else 20)
  hours=[sum(f' {hour:02}:' in slot['label'] for slot in local) for hour in range(4)]
  assert (0 in hours if kind=='gap' else 8 in hours)
 # Live staff scope and export restrictions.
 Person=people(f);limited=Person('Calendar member',['workspace.read','scheduling.read:assigned','scheduling.manage:assigned','workspace.data.export'])
 limited.call(root+'/appointments/'+sid,expected=404);assert limited.call(root+'/appointments')['items']==[]
 own=limited.call(root+'/types','POST',{'settings':settings});assert limited.call(root+'/types')['items'][0]['id']==own['id']
 limited.call(root+'/types/'+t['id'],expected=404)
 assert limited.call(base+'/exports/page','POST',{'collection':'appointments'})['items']==[]
 assert f.cli(['scheduling','get',sid])['id']==sid
 assert not f.mcp('scheduling_get',{'workspace_id':f.workspace,'id':sid}).get('isError',False)
 print('PASS: standalone scheduling, shared-host race exclusion, buffers, rescheduling/cancellation history, gap/fold timezone slots, access/export/CLI/MCP',flush=True)
 # Attach versioned Forms and independently activated CRM mapping.
 module('forms',True);module('crm',True)
 form=call(base+'/forms','POST',{'name':'Booking intake '+key(),'draft':{'fields':[{'key':'email','label':'Contact email','kind':'email','required':True},{'key':'name','label':'Contact name','kind':'short_text','required':True},{'key':'detail','label':'Project detail','kind':'long_text','required':True}]}},expected=201,key=key())
 form=call(base+'/forms/'+form['id']+'/publish','POST',{'expected_version':form['version']},key=key())
 pipeline=call(base+'/crm/pipelines','POST',{'name':'Bookings '+key()},expected=201,key=key());stage=call(base+'/crm/stages','POST',{'name':'Inquiry','parent_id':pipeline['id'],'outcome':'open'},expected=201,key=key())
 mapping={'name_fields':['name'],'email_field':'email','pipeline_id':pipeline['id'],'stage_id':stage['id'],'title_prefix':'Booking inquiry'}
 recipe=call(base+'/forms/'+form['id']+'/recipe','PUT',{'mapping':mapping},key=key());call(base+'/forms/'+form['id']+'/recipe/activation','POST',{'expected_version':recipe['version'],'active':True},key=key())
 sales=make_type(form_id=form['id'],crm=mapping,buffer_before=0,buffer_after=0)
 answers={'name':'Prospect','email':'prospect-'+key()+'@example.invalid','detail':'The approved intake answers.'}
 payload={'starts_at':at('16:00'),'attendee_name':'Calendar name','attendee_email':'calendar@example.invalid','form_version_id':form['published_version']['id'],'answers':answers}
 count=lambda table:int(sql(f"SELECT count(*) FROM {table} WHERE workspace_id='{f.workspace}'"))
 before=(count('scheduling.appointments'),count('forms.submissions'),count('scheduling.events'))
 pub('/types/'+sales['public_key']+'/book','POST',{**payload,'answers':{'name':'Invalid'}},expected=400)
 assert before==(count('scheduling.appointments'),count('forms.submissions'),count('scheduling.events'))
 # Failure at the durable event write rolls back appointment AND submission.
 sql("ALTER TABLE scheduling.events ADD CONSTRAINT fixture_event_failure CHECK(action<>'confirmed') NOT VALID")
 try:pub('/types/'+sales['public_key']+'/book','POST',payload,expected=503)
 finally:sql('ALTER TABLE scheduling.events DROP CONSTRAINT fixture_event_failure')
 assert before==(count('scheduling.appointments'),count('forms.submissions'),count('scheduling.events'))
 compose('stop','worker')
 code=secrets.token_hex(32);receipt=pub('/types/'+sales['public_key']+'/book','POST',payload,code=code);aid=receipt['id'];b=get(aid)
 assert b['recipe_state']=='queued' and b['intake']['answers']==answers
 assert pub('/types/'+sales['public_key']+'/book','POST',payload,code=code)['id']==aid
 assert int(sql(f"SELECT count(*) FROM forms.crm_recipe_runs WHERE submission_id='{b['submission_id']}'"))==0
 assert int(sql(f"SELECT count(*) FROM scheduling.events WHERE appointment_id='{aid}'"))==1
 module('crm',False);b=get(aid);assert b['job_state']=='blocked';compose('start','worker');module('crm',True);time.sleep(.5);assert get(aid)['job_state']=='blocked'
 call(root+'/appointments/'+aid+'/retry','POST',{'expected_version':b['version']},key=key());done=wait(aid);assert done['recipe_state']=='completed' and done['contact_id'] and done['opportunity_id']
 original=(done['submission_id'],done['contact_id'],done['opportunity_id'])
 contact=call(base+'/contacts/'+done['contact_id']);assert contact['display_name']=='Prospect'
 timeline=call(base+'/crm/timeline?anchor_kind=contact&anchor_id='+done['contact_id']);events=[x for x in timeline['items'] if x['event']['kind']=='appointment'];assert events and events[0]['event']['detail']['appointment']['intake']['answers']==answers
 # Retrying an already completed booking never creates more effects.
 compose('restart','worker');assert pub('/types/'+sales['public_key']+'/book','POST',payload,code=code)['id']==aid
 assert (get(aid)['submission_id'],get(aid)['contact_id'],get(aid)['opportunity_id'])==original
 customer(aid,code,'reschedule',starts_at=at('17:00'));customer(aid,code,'cancel');assert (get(aid)['submission_id'],get(aid)['contact_id'],get(aid)['opportunity_id'])==original
 # CRM failure keeps reservation; repair and explicit retry succeed once.
 compose('stop','worker');failed,fc,_=book(sales,at('18:00'),form_version_id=form['published_version']['id'],answers={**answers,'email':'repair-'+key()+'@example.invalid'})
 sql(f"UPDATE crm.opportunity_stages SET archived_at=now() WHERE id='{stage['id']}'")
 compose('start','worker');broken=wait(failed['id'],'blocked');assert broken['state']=='confirmed' and not broken['contact_id']
 sql(f"UPDATE crm.opportunity_stages SET archived_at=NULL WHERE id='{stage['id']}'")
 call(root+'/appointments/'+failed['id']+'/retry','POST',{'expected_version':broken['version']},key=key());assert wait(failed['id'])['recipe_state']=='completed'
 # Optional Forms disable pauses only its dependent booking workflow.
 compose('stop','worker');pending,pc,_=book(sales,at('19:00'),form_version_id=form['published_version']['id'],answers={**answers,'email':'pause-'+key()+'@example.invalid'})
 module('forms',False);assert get(pending['id'])['job_state']=='blocked';assert get(pending['id'])['intake'] is None
 module('forms',True);compose('start','worker');assert get(pending['id'])['job_state']=='blocked';call(root+'/appointments/'+pending['id']+'/retry','POST',{'expected_version':1},key=key());wait(pending['id'])
 print('PASS: invalid intake and event failure roll back atomically; embedded Forms recipe suppressed; durable CRM effects, timeline answers, disabled participants and recovery/retry',flush=True)
 # Reminder jobs survive restart; obsolete generations never send.
 compose('stop','worker');rem=make_type(reminder_minutes=60,buffer_before=0,buffer_after=0)
 obsolete,oc,_=book(rem,at('20:00'));customer(obsolete['id'],oc,'reschedule',starts_at=at('21:00'));customer(obsolete['id'],oc,'cancel')
 live,lc,live_input=book(rem,at('20:00'))
 sql(f"UPDATE suite.jobs SET available_at=now() WHERE id IN (SELECT job_id FROM scheduling.reminders WHERE appointment_id IN ('{obsolete['id']}','{live['id']}'))")
 sql(f"SELECT pgmq.set_vt('looski_jobs',message_id,0) FROM suite.jobs WHERE id IN (SELECT job_id FROM scheduling.reminders WHERE appointment_id IN ('{obsolete['id']}','{live['id']}'))")
 compose('start','worker')
 delivery_enabled=f.config['LOOSKI_MAIL_MODE']!='disabled'
 expected_counts='2|1' if delivery_enabled else '2|0'
 for _ in range(80):
  counts=sql(f"SELECT count(*) FILTER(WHERE skipped),count(*) FILTER(WHERE sent_at IS NOT NULL) FROM scheduling.reminders WHERE appointment_id IN ('{obsolete['id']}','{live['id']}')")
  live_job=sql(f"SELECT j.state||'|'||COALESCE(j.error_code,'') FROM suite.jobs j JOIN scheduling.reminders r ON r.job_id=j.id WHERE r.appointment_id='{live['id']}'")
  if counts==expected_counts and (delivery_enabled or live_job=='blocked|unavailable'):break
  time.sleep(.25)
 assert counts==expected_counts,(counts,live_job)
 if not delivery_enabled:assert live_job=='blocked|unavailable',live_job
 if f.config['LOOSKI_MAIL_MODE']=='local':
  inbox=json.loads(subprocess.check_output(['docker','compose','exec','-T','auth','wget','-qO-','http://mailpit:8025/api/v1/messages'],cwd=ROOT,env=f.environment))
  assert any(m['Subject']=='Appointment reminder: '+rem['settings']['name'] and any(to['Address']==live_input['attendee_email'] for to in m['To']) for m in inbox['messages']),'Reminder missing from private SMTP capture'
 assert int(sql(f"SELECT count(*) FROM scheduling.reminders WHERE appointment_id='{obsolete['id']}' AND sent_at IS NOT NULL"))==0
 module('scheduling',False);customer(live['id'],lc,expected=404);call(root+'/appointments',expected=403)
 print('PASS: durable delayed reminders, '+('delivery' if delivery_enabled else 'explicit block with email disabled')+', obsolete generation suppression, module pause',flush=True)
finally:
 compose('start','worker')
 for m,enabled in initial.items():module(m,enabled)

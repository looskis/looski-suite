#!/usr/bin/env python3
"""Time duration, concurrency, standalone scope, project totals and interfaces."""
import secrets,time
from concurrent.futures import ThreadPoolExecutor
from integration_fixture import Fixture
from fixture_people import people
f=Fixture();call,sql=f.call,f.sql;base='/api/v1/workspaces/'+f.workspace;root=base+'/time';key=lambda:secrets.token_hex(16)
def module(k,enabled):
 m=next(x for x in call(base+'/modules') if x['key']==k)
 if m['enabled']!=enabled:call(base+'/modules/'+k,'PATCH',{'enabled':enabled,'expected_version':m['version']},key=key())
def create(**changes):return call(root+'/entries','POST',{**payload,**changes},key=key())
def change(e,action,retry=None,expected=200):return call(root+'/entries/'+e['id']+'/transition','POST',{'expected_version':e['version'],'action':action},key=retry or key(),expected=expected)
initial={m['key']:m['enabled'] for m in call(base+'/modules')}
payload={'description':'Manual '+key(),'work_on':'2026-10-15','billable':True,'duration_ms':120123}
try:
 for m in initial:module(m,m=='time')
 e=create();assert e['duration_ms']==120123 and e['project_id'] is None
 k=key();timer=call(root+'/entries','POST',{**payload,'duration_ms':0,'start_timer':True},key=k)
 assert call(root+'/entries','POST',{**payload,'duration_ms':0,'start_timer':True},key=k)['id']==timer['id']
 call(root+'/entries','POST',{**payload,'duration_ms':0,'start_timer':True},key=key(),expected=409)
 time.sleep(.12);k=key();paused=change(timer,'pause',k);assert paused['duration_ms']>=120
 assert change(timer,'pause',k)['duration_ms']==paused['duration_ms'];time.sleep(.1)
 assert call(root+'/entries/'+timer['id'])['duration_ms']==paused['duration_ms']
 resumed=change(paused,'resume');time.sleep(.1);stopped=change(resumed,'stop');assert stopped['duration_ms']>=paused['duration_ms']+100
 change(stopped,'resume',expected=409)
 # One transition wins even with two independently valid retry keys.
 timer=create(duration_ms=0,start_timer=True)
 def pause(_):
  from urllib.request import Request,urlopen
  from urllib.error import HTTPError
  import json
  r=Request(f.base+root+'/entries/'+timer['id']+'/transition',data=json.dumps({'expected_version':timer['version'],'action':'pause'}).encode(),headers={'Authorization':'Bearer '+f.token,'Content-Type':'application/json','Idempotency-Key':key()},method='POST')
  try:response=urlopen(r)
  except HTTPError as error:response=error
  with response:return response.status
 with ThreadPoolExecutor(2) as pool:assert sorted(pool.map(pause,range(2)))==[200,409]
 change(call(root+'/entries/'+timer['id']),'stop')
 rows=call(root+'/entries?from_on=2026-10-15&through_on=2026-10-15&limit=100')['items'];report=call(root+'/report?from_on=2026-10-15&through_on=2026-10-15')
 assert report['duration_ms']==sum(x['duration_ms'] for x in rows) and report['billable_ms']==sum(x['duration_ms'] for x in rows if x['billable'])
 edit={**payload,'duration_ms':130123,'expected_version':e['version'],'archived':True};retry=key()
 changed=call(root+'/entries/'+e['id'],'PUT',edit,key=retry)
 assert changed['archived'] and changed['duration_ms']==130123
 assert call(root+'/entries/'+e['id'],'PUT',edit,key=retry)['version']==changed['version']
 call(root+'/entries/'+e['id'],'PUT',edit,key=key(),expected=409)
 assert call(root+'/report?from_on=2026-10-15&through_on=2026-10-15')['duration_ms']==report['duration_ms']-e['duration_ms']
 call(root+'/entries','POST',{**payload,'work_on':'2026-02-30'},key=key(),expected=400)
 b=call(root+'/budgets','POST',{'name':'October budget','from_on':'2026-10-01','through_on':'2026-10-31','budget_ms':3600000,'billable_only':True,'archived':False},key=key());assert b['budget_ms']==3600000
 Person=people(f);member=Person('Time member',['workspace.read','time.read:assigned','time.manage:assigned','workspace.data.export'])
 assert member.call(root+'/entries')['items']==[];member.call(root+'/entries/'+e['id'],expected=404)
 own=member.call(root+'/entries','POST',payload);assert member.call(root+'/report')['duration_ms']==120123
 exported=member.call(base+'/exports/page','POST',{'collection':'time_entries'})['items'];assert [x['id'] for x in exported]==[own['id']]
 member.call(root+'/budgets',expected=403)
 member.grants(['workspace.read']);member.call(root+'/entries/'+own['id'],expected=403)
 module('projects',True);p=call(base+'/projects','POST',{'name':'Timed project '+key()},key=key(),expected=201)
 linked=create(project_id=p['id'],duration_ms=60000)
 assert call(root+'/report?project_id='+p['id'])['duration_ms']==60000
 assert f.cli(['time','report','--project-id',p['id']])['duration_ms']==60000
 module('projects',False);assert call(root+'/entries/'+linked['id'])['project_id'] is None
 call(root+'/report?project_id='+p['id'],expected=403)
 assert f.cli(['time','get',e['id']])['id']==e['id']
 assert not f.mcp('time_get',{'workspace_id':f.workspace,'id':e['id']}).get('isError',False)
 module('time',False);call(root+'/entries',expected=403)
 print('PASS: standalone time, millisecond timer conservation, concurrent transition fence, scoped totals/exports, budgets, project authorization, CLI and MCP')
finally:
 for m,enabled in initial.items():module(m,enabled)

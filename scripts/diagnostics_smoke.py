#!/usr/bin/env python3
"""Assert local diagnostic ingestion and isolation using explicit test events."""
import json,secrets,subprocess,time
from integration_fixture import Fixture
f=Fixture();assert f.config['LOOSKI_DIAGNOSTICS_MODE']=='local'
def compose(*args,input=None):
 return subprocess.run(['docker','compose',*args],env=f.environment,capture_output=True,text=True,input=input,check=True).stdout.strip()
def sql(query):return compose('exec','-T','clickhouse','clickhouse-client','--query',query)
def wait_event(identifier):
 deadline=time.monotonic()+20
 while time.monotonic()<deadline:
  value=sql("SELECT count() FROM looski_diagnostics.events WHERE request_id='"+identifier+"'")
  if value!='0':return
  time.sleep(.25)
 raise AssertionError('Diagnostic test event did not arrive')
marker='PRIVATE-DIAGNOSTIC-PROBE-'+secrets.token_hex(12)
body={'display_name':marker,'notes':marker}
contact=f.call(f'/api/v1/workspaces/{f.workspace}/contacts','POST',body,expected=201,key=secrets.token_hex(16));request_id=f.request_id
wait_event(request_id)
event=json.loads(sql("SELECT * FROM looski_diagnostics.events WHERE request_id='"+request_id+"' LIMIT 1 FORMAT JSONEachRow"))
assert event['operation']=='POST /api/v1/workspaces/{workspace_id}/contacts',event['operation']
assert event['status']==201
assert f.sql("SELECT count(*) FROM suite.audit_events WHERE request_id='"+request_id+"'")=='1'
f.call('/api/v1/'+marker+'?token='+marker,expected=404);unknown=f.request_id
wait_event(unknown)
assert sql("SELECT operation FROM looski_diagnostics.events WHERE request_id='"+unknown+"'")=='GET unmatched'
assert sql("SELECT count() FROM looski_diagnostics.events WHERE position(operation,'"+marker+"')>0")=='0'
print('PASS: API event correlates with transactional audit; route templates and unmatched routes exclude private input',flush=True)
# The ingest key is passed through stdin to curl, never process arguments/logs.
key=(f.directory/'diagnostics-key').read_text().strip()
config='url = "http://clickhouse:8123/"\nheader = "X-ClickHouse-User: looski_ingest"\nheader = "X-ClickHouse-Key: '+key+'"\ndata = "SELECT count() FROM looski_diagnostics.events"\nsilent\noutput = "/dev/null"\nwrite-out = "%{http_code}"\n'
result=compose('exec','-T','api','curl','--config','-',input=config)
assert result=='500',result
# ClickHouse uses 500 with error 497 for insufficient grants; no key is printed.
print('PASS: ingest-only credential cannot read diagnostic rows',flush=True)
compose('stop','clickhouse')
try:
 started=time.monotonic()
 for _ in range(12):
  f.call(f'/api/v1/workspaces/{f.workspace}/contacts','POST',{'display_name':'Outage '+secrets.token_hex(4)},expected=201,key=secrets.token_hex(16))
 elapsed=time.monotonic()-started
 assert elapsed<12,elapsed
 assert f.call('/health/ready')['status']=='ok'
 time.sleep(3)
 print('PASS: twelve committed business writes during diagnostic outage in '+str(round(elapsed,3))+' seconds',flush=True)
finally:compose('start','clickhouse')
deadline=time.monotonic()+30
while time.monotonic()<deadline:
 try:
  sql('SELECT 1');break
 except subprocess.CalledProcessError:time.sleep(.5)
f.call('/api/v1/system');wait_event(f.request_id)
container=compose('ps','-q','clickhouse')
ports=json.loads(subprocess.check_output(['docker','inspect','--format','{{json .HostConfig.PortBindings}}',container],text=True))
assert not ports
print('PASS: ingestion resumes after ClickHouse restart; no diagnostic host ports',flush=True)
print('Diagnostic fixture footprint:',sql('SELECT count(),sum(bytes_on_disk) FROM system.parts WHERE active AND database=\'looski_diagnostics\''),flush=True)

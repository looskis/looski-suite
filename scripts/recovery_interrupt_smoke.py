#!/usr/bin/env python3
"""Recover the actual writer state after killing a disposable fixture backup."""
import json,os,secrets,signal,subprocess,time
from dev import configuration,ROOT
from recovery import Installation,recover_services
import recovery_archive as encryption
config=configuration();assert config['COMPOSE_PROJECT_NAME'].endswith('-identity-check')
i=Installation(config);original=set(i.running());directory=i.state/'recovery-fixture'
output=directory/('interrupted-'+secrets.token_hex(6)+'.age')
recipient=encryption.recipient(directory/'identity')
process=subprocess.Popen(['python3','scripts/recovery.py','create','--recipient',recipient,'--output',str(output)],cwd=ROOT,env={**os.environ,**config},stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
try:
 deadline=time.monotonic()+180
 while True:
  assert process.poll() is None,'Backup ended before interruption'
  assert time.monotonic()<deadline,'Backup did not reach maintenance'
  status=json.loads((i.state/'backup-status.json').read_text())
  if status.get('destination')==str(output.absolute()) and status.get('phase')=='capturing':break
  time.sleep(.05)
 os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=10)
 assert process.returncode==-signal.SIGKILL
 assert json.loads((i.state/'backup-status.json').read_text())['state']=='running'
 result=recover_services(i)
 assert result['restored'] and set(i.running())==original
 assert not output.exists()
 assert json.loads((i.state/'backup-status.json').read_text())['state']=='interrupted_services_restored'
 print('PASS: SIGKILL during real backup maintenance leaves a durable journal; recover-services restores the original healthy writers without publishing an archive',flush=True)
finally:
 if process.poll() is None:os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=10)
 if set(i.running())!=original:recover_services(i)

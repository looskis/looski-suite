#!/usr/bin/env python3
"""Late capture failure must preserve earlier archives and exact service state."""
import hashlib,secrets
from pathlib import Path
from unittest.mock import patch
from recovery import Installation,create
from dev import configuration
import recovery_archive as encryption
config=configuration();assert config['COMPOSE_PROJECT_NAME'].endswith('-identity-check')
i=Installation(config);directory=i.state/'recovery-fixture';identity=directory/'identity'
archive=directory/'looski-recovery-portable-identity-check.age'
before=hashlib.sha256(archive.read_bytes()).digest();running=set(i.running())
output=directory/('expected-failure-'+secrets.token_hex(6)+'.age')
try:
 with patch.object(encryption,'encrypt',side_effect=RuntimeError('Intentional late encryption failure')):
  try:create(i,output,encryption.recipient(identity))
  except RuntimeError as error:assert str(error)=='Intentional late encryption failure'
  else:raise AssertionError('Expected late failure')
 assert not output.exists() and hashlib.sha256(archive.read_bytes()).digest()==before
 assert set(i.running())==running
 import json
 assert json.loads((i.state/'backup-status.json').read_text())['state']=='failed_services_restored'
 print('PASS: late backup failure leaves prior authenticated archive unchanged, publishes no output and restores exact healthy service set',flush=True)
finally:
 assert set(i.running())==running

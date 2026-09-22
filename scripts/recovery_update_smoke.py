#!/usr/bin/env python3
"""Fail an update on a disposable restore, then recover its pre-update archive."""
import argparse,json,subprocess
from pathlib import Path
from recovery_restore import restore
from recovery import Installation
from dev import ROOT
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--archive',required=True,type=Path);parser.add_argument('--identity',required=True,type=Path)
parser.add_argument('--failed-target',required=True,type=Path);parser.add_argument('--project',required=True);parser.add_argument('--port',type=int,required=True)
args=parser.parse_args();target=args.failed_target.resolve();report_path=target/'recovery-report.json';report=json.loads(report_path.read_text())
assert target.parent==(ROOT/'.local/recoveries').resolve() and target.name.endswith('-identity-check')
assert args.project.endswith('-identity-check') and report['state'] in ('verified_isolated','preview_isolated')
from recovery_bundle import digest
assert report['source_archive_sha256']==digest(args.archive),'Use the exact pre-update archive restored to this disposable target'
i=Installation({'COMPOSE_PROJECT_NAME':target.name,'LOOSKI_HTTP_PORT':str(report['port']),'LOOSKI_PUBLIC_URL':'http://localhost:'+str(report['port']),'LOOSKI_MAIL_MODE':'disabled','LOOSKI_DIAGNOSTICS_MODE':'disabled'},target/'recipe')
i.compose+=['-f',str(target/'compose.recovery.json')]
services=[s for s in i.running() if s!='db']
if services:i.run('stop',*services)
image=json.loads((target/'compose.recovery.json').read_text())['services']['migrate']['image']
subprocess.run(['docker','run','--rm','--network',target.name+'_default','--mount','type=bind,src='+str(i.state/'secrets/migration_url')+',dst=/run/secrets/migration_url,readonly','--mount','type=bind,src='+str(ROOT/'scripts/migration_failure_probe.py')+',dst=/tmp/probe.py,readonly','--tmpfs','/locks','-e','PGSSLMODE=disable','--entrypoint','python3',image,'/tmp/probe.py','--stop-after-failure'],check=True)
assert i.sql("SELECT to_regclass('suite.recovery_migration_probe') IS NOT NULL")=='t'
assert i.sql("SELECT string_agg(version,',' ORDER BY version) FROM supabase_migrations.schema_migrations WHERE version LIKE '20990101%'")=='20990101000000'
report['state']='migration_failure_probe';report_path.write_text(json.dumps(report,indent=2))
result=restore(args.archive,args.identity,args.project,args.port)
replacement=Path(result['target']);fresh=Installation({'COMPOSE_PROJECT_NAME':args.project,'LOOSKI_HTTP_PORT':str(args.port),'LOOSKI_PUBLIC_URL':'http://localhost:'+str(args.port),'LOOSKI_MAIL_MODE':'disabled','LOOSKI_DIAGNOSTICS_MODE':'disabled'},replacement/'recipe');fresh.compose+=['-f',str(replacement/'compose.recovery.json')]
assert fresh.sql("SELECT to_regclass('suite.recovery_migration_probe') IS NULL")=='t'
assert fresh.sql("SELECT count(*) FROM supabase_migrations.schema_migrations WHERE version LIKE '20990101%'")=='0'
assert i.sql("SELECT to_regclass('suite.recovery_migration_probe') IS NOT NULL")=='t'
assert i.sql("SELECT string_agg(version,',' ORDER BY version) FROM supabase_migrations.schema_migrations WHERE version LIKE '20990101%'")=='20990101000000'
print('PASS: after a real partial update failure, the pre-update archive restores verified original data/schema to fresh resources while preserving the failed database',json.dumps(result),flush=True)
i.run('stop','db')

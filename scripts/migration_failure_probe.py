#!/usr/bin/env python3
"""Container-only partial-update probe; run only on a disposable recovery database.

Bind this script into the built migrator image; supply that isolated database's
migration_url secret and PGSSLMODE=disable. This adds two synthetic applied
versions and deliberately leaves the target unsuitable for production use.
"""
import subprocess,sys
from pathlib import Path
if not Path('/.dockerenv').exists() or not Path('/work/migration_integrity.py').is_file():
 raise SystemExit('This probe requires an isolated migrator container')
sys.path.insert(0,'/work')
import migration_integrity as guard
assert guard.database("SELECT count(*) FROM supabase_migrations.schema_migrations WHERE version LIKE '20990101%'")=='0','Use a fresh disposable recovery target'
migration=Path('/work/supabase/migrations')
a=migration/'20990101000000_partial_probe.sql';b=migration/'20990101000100_retry_probe.sql'
a.write_text('BEGIN; CREATE TABLE suite.recovery_migration_probe (id integer PRIMARY KEY); COMMIT;\n')
b.write_text('BEGIN; THIS IS DELIBERATELY INVALID; COMMIT;\n')
first=subprocess.run(['/bin/sh','/usr/local/bin/migrate'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
assert first.returncode!=0
assert guard.database("SELECT count(*) FROM supabase_migrations.schema_migrations WHERE version='20990101000000'")=='1'
assert guard.database("SELECT count(*) FROM supabase_migrations.schema_migrations WHERE version='20990101000100'")=='0'
print('PASS: first migration committed before a later migration failed',flush=True)
if '--stop-after-failure' in sys.argv:raise SystemExit(0)
b.write_text('BEGIN; INSERT INTO suite.recovery_migration_probe VALUES (1); COMMIT;\n')
second=subprocess.run(['/bin/sh','/usr/local/bin/migrate'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
assert second.returncode==0,'Corrected pending migration must remain retryable'
assert guard.database('SELECT count(*) FROM suite.recovery_migration_probe')=='1'
print('PASS: trusted attempted-source ledger permits corrected pending migration and retry',flush=True)
a.write_text(a.read_text()+'-- changed applied source\n')
third=subprocess.run(['python3','/work/migration_integrity.py','verify'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
assert third.returncode!=0
print('PASS: committed prefix remains immutable after retry',flush=True)

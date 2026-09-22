#!/usr/bin/env python3
"""Non-destructive migration-source drift check on the selected fixture DB."""
import tempfile,shutil
from pathlib import Path
from dev import configuration,ROOT
from recovery import Installation,verify_migrations
config=configuration();assert config['COMPOSE_PROJECT_NAME'].endswith('-identity-check')
i=Installation(config)
with tempfile.TemporaryDirectory(prefix='looski-migration-source-') as directory:
 root=Path(directory)
 shutil.copytree(ROOT/'supabase/migrations',root/'supabase/migrations')
 (root/'infra/migrator').mkdir(parents=True)
 shutil.copyfile(ROOT/'infra/migrator/baseline.json',root/'infra/migrator/baseline.json')
 verify_migrations(i,root)
 path=next((root/'supabase/migrations').glob('*.sql'));path.write_bytes(path.read_bytes()+b'\n-- changed historical source\n')
 try:verify_migrations(i,root)
 except ValueError as error:assert 'removed or changed' in str(error)
 else:raise AssertionError('Modified applied migration accepted for backup')
 path.unlink()
 try:verify_migrations(i,root)
 except ValueError as error:assert 'removed or changed' in str(error)
 else:raise AssertionError('Missing applied migration accepted for backup')
print('PASS: recovery accepts canonical source and rejects changed/missing applied migrations before publication')

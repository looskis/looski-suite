#!/usr/bin/env python3
"""Generate both contracts into temporary files; --check never edits sources."""
import argparse
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
with tempfile.TemporaryDirectory() as directory:
    spec = Path(directory) / 'openapi.json'
    types = Path(directory) / 'schema.d.ts'
    spec.write_bytes(subprocess.check_output(['cargo', 'run', '--locked', '--quiet', '-p', 'looski', '--', 'openapi'], cwd=ROOT))
    subprocess.run(['pnpm', 'exec', 'openapi-typescript', str(spec), '-o', str(types)], cwd=ROOT, check=True)
    for source, target in [(spec, ROOT / 'contracts/openapi.json'), (types, ROOT / 'apps/web/src/lib/api/schema.d.ts')]:
        if args.check:
            if not target.exists() or source.read_bytes() != target.read_bytes():
                raise SystemExit(f'Contract drift: run pnpm api:generate ({target.relative_to(ROOT)})')
        else:
            target.write_bytes(source.read_bytes())

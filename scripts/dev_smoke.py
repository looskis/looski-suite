#!/usr/bin/env python3
"""The running development stack serves this checkout: ready API, live worker, current
schema, and a real source change reloading the API. Changes no business data."""
import json
import os
import subprocess
import time
from urllib.error import HTTPError, URLError
from urllib.request import urlopen
from dev import ROOT, configuration

config = configuration(True)
base = f"http://localhost:{config['LOOSKI_HTTP_PORT']}"
compose = ['docker', 'compose', '-f', 'compose.yaml', '-f', 'compose.dev.yaml']
environment = {**os.environ, **config}


def status(path):
    try:
        with urlopen(base + path, timeout=5) as response:
            return response.status
    except HTTPError as failure:
        return failure.status
    except (URLError, OSError):
        return 0


def compose_output(*arguments):
    return subprocess.check_output([*compose, *arguments], cwd=ROOT, env=environment, text=True)


def wait_for(predicate, what, seconds):
    deadline = time.monotonic() + seconds
    while not predicate():
        if time.monotonic() > deadline:
            logs = compose_output('logs', '--tail', '15', 'api', 'worker')
            raise SystemExit(f'Timed out waiting for {what}\n{logs}')
        time.sleep(1)


def health(service):
    state = json.loads(compose_output('ps', '--format', 'json', service) or '{}')
    return state.get('Health')


def listening():
    return compose_output('logs', 'api').count('API listening')


expected = max(int(path.name.split('_')[0]) for path in (ROOT / 'supabase/migrations').glob('*.sql'))
applied = compose_output('exec', '-T', 'db', 'psql', '-U', 'postgres', '-d', 'postgres', '-Atc',
                         'SELECT max(version) FROM supabase_migrations.schema_migrations').strip()
assert int(applied) == expected, f'Database is at migration {applied}, checkout has {expected}: run make dev-migrate'
print('PASS: every migration in this checkout is applied')
wait_for(lambda: status('/health/ready') == 200, 'API readiness', 300)
print('PASS: API ready')
wait_for(lambda: health('worker') == 'healthy', 'a healthy worker', 300)
print('PASS: worker healthy')
assert status('/') == 200 or status('/') in (302, 303), 'Web is not serving'
print('PASS: web serving through the gateway')
assert status('/api/v1/workspaces') == 401, 'Business routes must still require a session'
print('PASS: business routes closed without a session')
before = listening()
os.utime(ROOT / 'crates/looski/src/lib.rs')
wait_for(lambda: listening() > before, 'the API to rebuild and restart after a source change', 600)
wait_for(lambda: status('/health/ready') == 200, 'API readiness after reload', 60)
wait_for(lambda: health('worker') == 'healthy', 'a healthy worker after reload', 300)
print('PASS: a source change rebuilt and restarted the API and worker')

#!/usr/bin/env python3
"""Exercise the running scaffold without creating business records."""
import json
import os
import selectors
import subprocess
import tempfile
from urllib.error import HTTPError
from urllib.request import urlopen

from dev import ROOT, configuration

config = configuration()
base = f"http://localhost:{config['LOOSKI_HTTP_PORT']}"
environment = {**os.environ, **config}


def request(path, status=200):
    try:
        response = urlopen(base + path, timeout=10)
    except HTTPError as error:
        response = error
    with response:
        assert response.status == status, (path, response.status, status)
        return response.read().decode()


system = json.loads(request('/api/v1/system'))
assert system['stage'] == 'scaffold'
assert json.loads(request('/health/ready'))['status'] == 'ok'
assert 'Connected to Looski Suite' in request('/')  # Present in SSR HTML, without JS.
assert json.loads(request('/api/v1/openapi.json')) == json.loads((ROOT / 'contracts/openapi.json').read_text())
request('/auth/v1/health')
for path in ['/auth/v1/admin/users', '/auth/v1/token', '/rest/v1/contacts', '/api/v1/contacts']:
    request(path, 404)
request('/api/v1/workspaces', 401)
assert 'Your business starts here.' in request('/login')

binary = ROOT / 'target/debug/looski'
status = subprocess.check_output([str(binary), 'status', '--api-url', base], text=True)
assert json.loads(status) == system

# Real stdio initialization, discovery, and tool call. stdout must remain JSON-RPC.
with tempfile.TemporaryFile() as errors:
    process = subprocess.Popen([str(binary), 'mcp', '--api-url', base], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=errors, text=True, bufsize=1)
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)

    def rpc(method, params=None, identifier=None):
        message = {'jsonrpc': '2.0', 'method': method}
        if params is not None:
            message['params'] = params
        if identifier is not None:
            message['id'] = identifier
        process.stdin.write(json.dumps(message) + '\n')
        process.stdin.flush()
        if identifier is None:
            return
        while True:
            assert selector.select(10), f'MCP timeout: {method}'
            reply = json.loads(process.stdout.readline())
            if reply.get('id') == identifier:
                assert 'error' not in reply, reply
                return reply['result']

    try:
        initialized = rpc('initialize', {'protocolVersion': '2025-11-25', 'capabilities': {},
                           'clientInfo': {'name': 'looski-smoke', 'version': '0.1.0'}}, 1)
        assert initialized['serverInfo']['name'] == 'looski-suite'
        assert initialized['serverInfo']['version'] == system['version']
        rpc('notifications/initialized')
        assert {tool['name'] for tool in rpc('tools/list', {}, 2)['tools']} == {'system_status', 'list_workspaces'}
        result = rpc('tools/call', {'name': 'system_status', 'arguments': {}}, 3)
        assert not result.get('isError')
        assert json.loads(result['content'][0]['text']) == system
        access = rpc('tools/call', {'name': 'list_workspaces', 'arguments': {}}, 4)
        if os.environ.get('LOOSKI_ACCESS_TOKEN_FILE'):
            expected = subprocess.check_output([str(binary), 'workspaces', '--api-url', base], text=True)
            assert not access.get('isError')
            assert json.loads(access['content'][0]['text']) == json.loads(expected)
        else:
            assert access['isError']
    finally:
        process.stdin.close()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.terminate()
            process.wait(timeout=5)
        selector.close()

# Verify PGMQ and a representative write share transaction rollback in real PG.
# Temporary table and queue changes are all rolled back; no business data touched.
sql = """
BEGIN;
CREATE TEMP TABLE scaffold_atomic (id integer);
SELECT pgmq.create('scaffold_atomic_probe');
SAVEPOINT before_operation;
INSERT INTO scaffold_atomic VALUES (1);
SELECT pgmq.send('scaffold_atomic_probe', '{"probe":true}'::jsonb);
ROLLBACK TO before_operation;
DO $$ BEGIN
  IF EXISTS (SELECT FROM scaffold_atomic) OR EXISTS (SELECT FROM pgmq.q_scaffold_atomic_probe) THEN
    RAISE EXCEPTION 'Atomic rollback failed';
  END IF;
  IF NOT EXISTS (SELECT FROM auth.schema_migrations) THEN
    RAISE EXCEPTION 'Supabase Auth migrations did not run';
  END IF;
  IF has_schema_privilege('anon', 'suite', 'USAGE') OR has_schema_privilege('authenticated', 'suite', 'USAGE') THEN
    RAISE EXCEPTION 'Business schema is accessible outside Rust';
  END IF;
  IF has_schema_privilege('looski_runtime', 'suite', 'CREATE') THEN
    RAISE EXCEPTION 'Runtime role can migrate schema';
  END IF;
END $$;
ROLLBACK;
"""
subprocess.run(['docker', 'compose', 'exec', '-T', 'db', 'psql', '-U', 'postgres', '-d', 'postgres',
                '-v', 'ON_ERROR_STOP=1'], input=sql, text=True, check=True, cwd=ROOT, env=environment,
               stdout=subprocess.DEVNULL)
print('PASS: SSR, API/OpenAPI, CLI, MCP, readiness, private routes, Auth migrations, role isolation, PGMQ rollback')

#!/usr/bin/env python3
"""Real identity/authorization checks; only run on a dedicated *-identity-check installation.

Retains a private fixture for repeat runs; never uses the primary installation.
"""
import http.cookiejar
import json
import os
import secrets
import subprocess
from concurrent.futures import ThreadPoolExecutor
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, build_opener, HTTPCookieProcessor, HTTPRedirectHandler

from dev import ROOT, configuration

config = configuration()
assert config['COMPOSE_PROJECT_NAME'].endswith('-identity-check'), 'Use a dedicated identity-check project'
environment = {**os.environ, **config}
base = config['LOOSKI_PUBLIC_URL']
compose = ['docker', 'compose']


def sql(statement):
    return subprocess.check_output([*compose, 'exec', '-T', 'db', 'psql', '-U', 'postgres',
                                   '-d', 'postgres', '-At', '-v', 'ON_ERROR_STOP=1'],
                                  input=statement, text=True, cwd=ROOT, env=environment).strip()


fixture = ROOT / '.local' / config['COMPOSE_PROJECT_NAME'] / 'identity-fixture.json'
if not fixture.exists():
    assert sql('SELECT bootstrapped_at IS NOT NULL FROM suite.installation') == 'f', 'Existing owner; refusing to create fixture'
    with fixture.open('x') as stream:
        os.chmod(stream.name, 0o600)
        json.dump({'email': f'owner-{secrets.token_hex(5)}@example.invalid',
                   'password': secrets.token_urlsafe(24), 'display_name': 'Integration owner',
                   'workspace_name': 'Integration workspace'}, stream)
owner = json.loads(fixture.read_text())


def bootstrap(credentials=owner):
    return subprocess.run([*compose, 'exec', '-T', 'api', '/bin/sh',
                           '/usr/local/bin/looski-entrypoint', 'bootstrap-owner'],
                          input=json.dumps(credentials), text=True, capture_output=True,
                          cwd=ROOT, env=environment)


if sql('SELECT bootstrapped_at IS NOT NULL FROM suite.installation') == 'f':
    # Inject a suite failure after upstream Auth creates the account. Every
    # suite write must roll back, and only the correct credentials may resume.
    sql("""CREATE FUNCTION suite.identity_probe_fail() RETURNS trigger LANGUAGE plpgsql AS $$
      BEGIN RAISE EXCEPTION 'integration fault'; END $$;
      CREATE TRIGGER identity_probe_fail BEFORE INSERT ON suite.audit_events
      FOR EACH ROW EXECUTE FUNCTION suite.identity_probe_fail();""")
    try:
        assert bootstrap().returncode != 0
        assert sql('SELECT count(*) FROM suite.users') == '0'
        assert sql('SELECT count(*) FROM suite.workspaces') == '0'
        assert sql('SELECT bootstrapped_at IS NOT NULL FROM suite.installation') == 'f'
    finally:
        sql('DROP TRIGGER identity_probe_fail ON suite.audit_events; DROP FUNCTION suite.identity_probe_fail();')
    assert bootstrap({**owner, 'password': secrets.token_urlsafe(24)}).returncode != 0
    with ThreadPoolExecutor(max_workers=2) as executor:
        results = list(executor.map(lambda _: bootstrap(), range(2)))
    assert sum(r.returncode == 0 for r in results) == 1, [r.stderr for r in results]
assert bootstrap().returncode != 0, 'Bootstrap reopened'
assert sql("SELECT count(*) FROM suite.audit_events WHERE action = 'installation.bootstrapped'") == '1'


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


jar = http.cookiejar.CookieJar()
browser = build_opener(HTTPCookieProcessor(jar), NoRedirect())


def request(path, status=200, data=None, headers=None):
    req = Request(base + path, data=data, headers={'Accept': 'text/html', **(headers or {})})
    try:
        response = browser.open(req, timeout=15)
    except HTTPError as failure:
        response = failure
    assert response.status == status, (path, response.status, status)
    with response:
        return response.read().decode(), response.headers


request('/workspaces', 303)
form = urlencode({'email': owner['email'], 'password': owner['password']}).encode()
request('/login', 403, form, {'Origin': 'https://attacker.invalid'})
_, headers = request('/login', 303, form, {'Origin': base})
assert headers['Location'] == '/workspaces'
assert 'no-store' in headers['Cache-Control']
cookies = headers.get_all('Set-Cookie')
assert len(cookies) == 2 and all('HttpOnly' in c and 'SameSite=Lax' in c for c in cookies)
assert 'Integration workspace' in request('/workspaces')[0]
token = next(c.value for c in jar if c.name == 'looski_access')
authorization = {'Authorization': 'Bearer ' + token}
workspaces = json.loads(request('/api/v1/workspaces', headers=authorization)[0])
assert len(workspaces) == 1 and workspaces[0]['name'] == owner['workspace_name']
request('/api/v1/workspaces', 401)  # Browser cookies never authenticate the raw API.

# CLI shares the same operation; only a file path, never a secret, appears in argv.
token_file = fixture.parent / 'access-token'
with token_file.open('w') as stream:
    os.chmod(stream.name, 0o600)
    stream.write(token)
try:
    result = subprocess.check_output([str(ROOT / 'target/debug/looski'), 'workspaces',
                                      '--api-url', base, '--token-file', str(token_file)], text=True)
    assert json.loads(result) == workspaces
    subprocess.run(['python3', 'scripts/smoke.py'], cwd=ROOT,
                   env={**environment, 'LOOSKI_ACCESS_TOKEN_FILE': str(token_file)}, check=True)
finally:
    token_file.unlink()

# This fixture installation contains only our owner. Each revocation must take
# effect with the same still-valid session; restore immediately even on failure.
for deny, restore in [
    ("UPDATE suite.memberships SET state='removed'", "UPDATE suite.memberships SET state='active'"),
    ("UPDATE suite.users SET state='suspended'", "UPDATE suite.users SET state='active'"),
    ("UPDATE suite.workspaces SET state='suspended'", "UPDATE suite.workspaces SET state='active'"),
    ("UPDATE suite.roles SET archived_at=now()", "UPDATE suite.roles SET archived_at=NULL"),
    ("UPDATE suite.auth_identities SET disabled_at=now()", "UPDATE suite.auth_identities SET disabled_at=NULL"),
    ("DELETE FROM suite.role_permissions", "INSERT INTO suite.role_permissions SELECT workspace_id,id,'workspace.read' FROM suite.roles"),
]:
    try:
        sql(deny)
        assert json.loads(request('/api/v1/workspaces', headers=authorization)[0]) == []
    finally:
        sql(restore)

sql("""
BEGIN;
DO $$ DECLARE other_workspace uuid; other_role uuid; existing_workspace uuid; member uuid;
BEGIN
  SELECT workspace_id,id INTO existing_workspace,member FROM suite.memberships LIMIT 1;
  INSERT INTO suite.workspaces(name) VALUES ('Other workspace') RETURNING id INTO other_workspace;
  INSERT INTO suite.roles(workspace_id,key,name) VALUES (other_workspace,'test','Test') RETURNING id INTO other_role;
  BEGIN
    INSERT INTO suite.membership_roles VALUES (existing_workspace,member,other_role);
    RAISE EXCEPTION 'Cross-workspace role assignment accepted';
  EXCEPTION WHEN foreign_key_violation THEN NULL;
  END;
END $$;
ROLLBACK;
""")

# Expired/missing access cookie uses the rotating refresh cookie server-side.
for cookie in list(jar):
    if cookie.name == 'looski_access':
        jar.clear(cookie.domain, cookie.path, cookie.name)
assert 'Integration workspace' in request('/workspaces')[0]
request('/logout', 403, b'', {'Origin': 'https://attacker.invalid'})
request('/logout', 303, b'', {'Origin': base})
assert not list(jar), 'Logout retained session cookies'
request('/workspaces', 303)
print('PASS: owner bootstrap/replay, browser sign-in/refresh/logout, CSRF, API/CLI/MCP, current permissions, workspace constraints, audit')

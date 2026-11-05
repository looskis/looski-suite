#!/usr/bin/env python3
"""Exercise the built web OAuth boundary against an isolated mock Auth server.

Run `pnpm build` first. No installation, Google account, or real credentials are used.
"""
import base64
import hashlib
from http.cookies import SimpleCookie
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time
from urllib.error import HTTPError, URLError
from urllib.parse import parse_qs, urlencode, urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener

ROOT = Path(__file__).resolve().parents[1]
flows = {}
calls = []
auth_mode = 'ok'


def encode(value):
    return base64.urlsafe_b64encode(value).decode().rstrip('=')


class Auth(BaseHTTPRequestHandler):
    def log_message(self, *args): pass

    def do_GET(self):
        query = parse_qs(urlsplit(self.path).query)
        assert urlsplit(self.path).path == '/authorize'
        assert query['provider'] == ['google']
        assert query['prompt'] == ['select_account']
        assert query['code_challenge_method'] == ['s256']
        assert query['redirect_to'] == [origin + '/auth/callback']
        code = 'code-' + str(len(flows))
        flows[code] = query['code_challenge'][0]
        calls.append(('start', code))
        self.send_response(302 if auth_mode != 'unavailable' else 503)
        self.send_header('Location', ('https://accounts.google.com/o/oauth2/v2/auth?state=' if auth_mode != 'unsafe' else 'https://attacker.invalid/?state=') + code)
        self.end_headers()

    def do_POST(self):
        assert self.path == '/token?grant_type=pkce'
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        calls.append(('exchange', body['auth_code']))
        expected = flows.get(body['auth_code'])
        valid = expected and expected == encode(hashlib.sha256(body['code_verifier'].encode()).digest())
        if valid: del flows[body['auth_code']]
        self.send_response(200 if valid else 400)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps({'access_token': 'mock-access', 'refresh_token': 'mock-refresh', 'expires_in': 3600, 'provider_token': 'must-not-reach-browser'} if valid else {'error': 'invalid_grant'}).encode())


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *args): return None


browser = build_opener(NoRedirect())


def request(path, status=200, cookie=None, post=False, request_origin=None):
    headers = {'Accept': 'text/html'}
    if cookie: headers['Cookie'] = cookie
    if post:
        headers['Origin'] = request_origin or origin
        headers['Content-Type'] = 'application/x-www-form-urlencoded'
    req = Request(base + path, data=b'' if post else None, headers=headers)
    try: response = browser.open(req, timeout=10)
    except HTTPError as failure: response = failure
    with response:
        body = response.read().decode()
        assert response.status == status, (path, response.status, status, body[:200])
        assert 'must-not-reach-browser' not in body
        return body, response.headers


def start(next_path='/workspaces'):
    _, headers = request('/auth/google?' + urlencode({'next': next_path}), 303, post=True)
    assert headers['Location'].startswith('https://accounts.google.com/')
    assert headers['Referrer-Policy'] == 'no-referrer'
    assert headers['Cache-Control'] == 'no-store'
    cookie = SimpleCookie(); cookie.load(headers['Set-Cookie'])
    name = '__Host-looski_google' if origin.startswith('https:') else 'looski_google'
    item = cookie[name]
    assert item['httponly'] and item['samesite'] == 'Lax' and item['path'] == '/'
    assert bool(item['secure']) == origin.startswith('https:')
    assert item['max-age'] == '600'
    code = parse_qs(urlsplit(headers['Location']).query)['state'][0]
    return name + '=' + item.value, code


auth = ThreadingHTTPServer(('127.0.0.1', 0), Auth)
threading.Thread(target=auth.serve_forever, daemon=True).start()
try:
    for enabled, https in [(False, False), (True, False), (True, True)]:
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
        base = 'http://127.0.0.1:' + str(port)
        origin = 'https://suite.example.test' if https else base
        environment = {**os.environ, 'HOST': '127.0.0.1', 'PORT': str(port), 'ORIGIN': origin,
                       'LOOSKI_PUBLIC_URL': origin, 'LOOSKI_GOOGLE_AUTH_ENABLED': str(enabled).lower(),
                       'LOOSKI_AUTH_INTERNAL_URL': 'http://127.0.0.1:' + str(auth.server_port)}
        with tempfile.TemporaryFile() as output:
            process = subprocess.Popen(['node', str(ROOT / 'apps/web/build/index.js')], env=environment, stdout=output, stderr=output)
            try:
                for attempt in range(100):
                    try:
                        body, _ = request('/login'); break
                    except URLError:
                        if process.poll() is not None:
                            output.seek(0); raise AssertionError(output.read().decode())
                        time.sleep(.05)
                else: raise AssertionError('Web did not start')
                assert ('Continue with Google' in body) == enabled
                if not enabled:
                    request('/auth/google', 404, post=True)
                    _, headers = request('/auth/callback?code=untrusted', 303)
                    assert headers['Location'] == '/login?google=failed#'
                    continue
                request('/auth/google', 405)
                request('/auth/google', 403, post=True, request_origin='https://attacker.invalid')
                before = len(calls)
                _, headers = request('/auth/callback?code=unsolicited', 303)
                assert len(calls) == before and 'looski_access' not in str(headers)
                for target in ['/cli/authorize?request=test', '/invitations/accept', '//attacker.invalid', '/auth/google']:
                    cookie, code = start(target)
                    _, headers = request('/auth/callback?' + urlencode({'code': code}), 303, cookie)
                    assert headers['Location'] == (target if target.startswith(('/cli/', '/invitations/')) else '/workspaces')
                    assert headers['Referrer-Policy'] == 'no-referrer'
                    assert headers['Cache-Control'] == 'no-store'
                    cookies = headers.get_all('Set-Cookie')
                    assert any('looski_access=' in c and 'HttpOnly' in c for c in cookies)
                    assert any('looski_refresh=' in c and 'HttpOnly' in c for c in cookies)
                    assert 'must-not-reach-browser' not in str(headers)
                    _, headers = request('/auth/callback?' + urlencode({'code': code}), 303, cookie)
                    assert headers['Location'].startswith('/login?google=failed')
                    assert 'looski_access' not in str(headers)
                cookie, code = start()
                other_cookie, _ = start()
                _, headers = request('/auth/callback?code=' + code, 303, other_cookie)
                assert 'looski_access' not in str(headers)
                # Expired/corrupt state and provider cancellation never exchange a code.
                name, value = cookie.split('=', 1)
                pending = json.loads(base64.urlsafe_b64decode(value + '=' * (-len(value) % 4)))
                pending['expires'] = 1
                for invalid in ['invalid', encode(b'null'), encode(json.dumps(pending).encode())]:
                    before = len(calls)
                    _, headers = request('/auth/callback?code=' + code, 303, name + '=' + invalid)
                    assert len(calls) == before and 'looski_access' not in str(headers)
                before = len(calls)
                _, headers = request('/auth/callback?error=access_denied&code=' + code, 303, cookie)
                assert len(calls) == before and 'looski_access' not in str(headers)
                for auth_mode in ['unsafe', 'unavailable']:
                    _, headers = request('/auth/google', 503, post=True)
                    assert not headers.get_all('Set-Cookie')
                auth_mode = 'ok'
            finally:
                process.terminate()
                try: process.wait(timeout=10)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
finally:
    auth.shutdown(); auth.server_close()
print('PASS: Google OAuth enablement, CSRF, PKCE browser binding/replay, callbacks, safe return paths, cookie flags, and upstream failures (mock Auth)')

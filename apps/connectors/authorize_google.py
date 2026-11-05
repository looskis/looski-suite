#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""Local OAuth setup. Register http://localhost:58911/callback on a dedicated web client."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import secrets
from urllib.parse import parse_qs, urlencode, urlsplit
from urllib.request import Request, urlopen
from providers import Google, b64

SCOPES = ['https://www.googleapis.com/auth/gmail.readonly', 'https://www.googleapis.com/auth/gmail.send',
          'https://www.googleapis.com/auth/calendar.events']
REDIRECT = 'http://localhost:58911/callback'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--client', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--email', required=True)
    parser.add_argument('--import-after', required=True, help='YYYY/MM/DD, inclusive initial mailbox import boundary')
    args = parser.parse_args()
    os.umask(0o077)
    if args.output.exists(): raise SystemExit('Output already exists; choose a new private file for reconnection')
    client = json.loads(args.client.read_text())
    client = client.get('web', client.get('installed', client))
    state, verifier = secrets.token_urlsafe(32), secrets.token_urlsafe(64)
    params = dict(client_id=client['client_id'], redirect_uri=REDIRECT, response_type='code',
                  scope=' '.join(SCOPES), access_type='offline', prompt='consent', state=state,
                  code_challenge=b64(hashlib.sha256(verifier.encode()).digest()), code_challenge_method='S256',
                  login_hint=args.email)
    complete = False
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args): pass
        def do_GET(self):
            nonlocal complete
            query = parse_qs(urlsplit(self.path).query)
            if urlsplit(self.path).path != '/callback' or not secrets.compare_digest(query.get('state', [''])[0], state):
                self.send_error(400, 'Invalid authorization state'); return
            try:
                if 'error' in query: raise ValueError('Authorization declined')
                data = urlencode(dict(client_id=client['client_id'], client_secret=client['client_secret'],
                                      redirect_uri=REDIRECT, grant_type='authorization_code',
                                      code=query['code'][0], code_verifier=verifier)).encode()
                with urlopen(Request('https://oauth2.googleapis.com/token', data=data), timeout=20) as response:
                    token = json.load(response)
                granted = set(token.get('scope', '').split())
                if not set(SCOPES).issubset(granted): raise ValueError('Required scopes not granted')
                credentials = dict(client_id=client['client_id'], client_secret=client['client_secret'], refresh_token=token['refresh_token'])
                google = Google(credentials)
                profile = google.call('gmail/v1/users/me/profile')
                if profile['emailAddress'].lower() != args.email.lower(): raise ValueError('Wrong Google account')
                google.call('calendar/v3/calendars/primary/events?maxResults=1')
                config = dict(provider='gmail', address=args.email, installation_id='gmail-' + secrets.token_hex(16),
                              api_token=secrets.token_urlsafe(48), google=credentials, calendar_enabled=True,
                              calendar='primary', conflict_calendars=['primary'], import_after=args.import_after)
                args.output.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                with args.output.open('x') as output: json.dump(config, output, indent=2)
                os.chmod(args.output, 0o600)
                complete = True
                body = b'Google connected. You can close this window. No messages were sent.'
                self.send_response(200); self.send_header('Content-Type', 'text/plain'); self.send_header('Cache-Control', 'no-store')
                self.send_header('Referrer-Policy', 'no-referrer'); self.end_headers(); self.wfile.write(body)
            except Exception:
                self.send_error(400, 'Connection failed. Check account, scopes and OAuth client configuration; restart setup.')
    server = HTTPServer(('127.0.0.1', 58911), Handler)
    server.timeout = 1
    print('Open this URL on this computer to authorize Gmail and Calendar:', flush=True)
    print('https://accounts.google.com/o/oauth2/v2/auth?' + urlencode(params), flush=True)
    try:
        import time
        deadline = time.monotonic() + 900
        while not complete and time.monotonic() < deadline: server.handle_request()
    finally: server.server_close()
    if not complete: raise SystemExit('Authorization expired; run setup again')
    print('Private connector configuration saved; start server.py to begin synchronization.')


if __name__ == '__main__': main()

#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""One process per mailbox; bind loopback or place behind an authenticated TLS proxy."""
import argparse
import hmac
import fcntl
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import threading
from urllib.parse import parse_qs, urlsplit
from store import Store
from providers import Gmail, Imap, Google
from calendar_provider import Calendar


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', required=True, type=Path)
    parser.add_argument('--port', type=int, default=58910)
    args = parser.parse_args()
    os.umask(0o077)
    if args.config.stat().st_mode & 0o077: raise SystemExit('Config must have mode 0600')
    journal_lock = (args.config.parent / 'journal.lock').open('a')
    try: fcntl.flock(journal_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError: raise SystemExit('This mailbox connector is already running')
    config = json.loads(args.config.read_text())
    if len(config['api_token']) < 32: raise SystemExit('Use a random API token of at least 32 characters')
    if config['provider'] not in ('gmail', 'imap'): raise SystemExit('Unknown provider')
    if config['provider'] == 'gmail':
        from datetime import datetime
        datetime.strptime(config['import_after'], '%Y/%m/%d')
    store = Store(str(args.config.parent / 'journal.sqlite'), config['installation_id'])
    google = Google(config['google']) if config['provider'] == 'gmail' else None
    provider = Gmail(config, store, google) if google else Imap(config, store)
    calendar = Calendar(Google(config['google']), store, config) if google and config.get('calendar_enabled') else None
    lock = threading.Lock()
    stop = threading.Event()
    def worker():
        while not stop.is_set():
            try:
                provider.poll()
                store.set('health', {'state': 'ready'})
            except Exception:
                # Never log provider responses, mail, addresses or credentials.
                store.set('health', {'state': 'provider_unavailable'})
            for _ in range(20):
                store.drain(provider)
            stop.wait(15)
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args): pass
        def respond(self, status, value):
            raw = json.dumps(value).encode()
            self.send_response(status)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Cache-Control', 'no-store')
            self.send_header('Content-Length', str(len(raw)))
            self.end_headers()
            self.wfile.write(raw)
        def handle_request(self):
            if not hmac.compare_digest(self.headers.get('Authorization', ''), 'Bearer ' + config['api_token']):
                return self.respond(401, {'error': 'unauthorized'})
            parts = urlsplit(self.path)
            if self.command == 'GET':
                if parts.path == '/v1/capabilities':
                    return self.respond(200, dict(version=1, installation_id=store.installation, idempotent_send=True, calendar=bool(calendar)))
                if parts.path == '/v1/health': return self.respond(200, store.get('health', {'state': 'starting'}))
                if parts.path == '/v1/events':
                    query = parse_qs(parts.query)
                    since = int(query.get('since', ['0'])[0]); limit = int(query.get('limit', ['50'])[0])
                    if since < 0 or not 1 <= limit <= 50: raise ValueError('Invalid event cursor')
                    return self.respond(200, store.events(since, limit))
            elif self.command == 'POST':
                size = int(self.headers.get('Content-Length', '0'))
                if not 0 < size <= 100_000: return self.respond(413, {'error': 'invalid_size'})
                data = json.loads(self.rfile.read(size))
                if parts.path == '/v1/messages': return self.respond(202, store.enqueue(data))
                if calendar and parts.path.startswith('/v1/calendar/'):
                    with lock:
                        if parts.path == '/v1/calendar/busy': return self.respond(200, calendar.busy(data))
                        if parts.path == '/v1/calendar/appointments': return self.respond(200, calendar.apply(data))
            self.respond(404, {'error': 'not_found'})
        def do_GET(self):
            try: self.handle_request()
            except (ValueError, KeyError, TypeError): self.respond(409, {'error': 'invalid_or_conflicting_request'})
            except Exception: self.respond(503, {'error': 'provider_unavailable'})
        do_POST = do_GET
    worker_thread = threading.Thread(target=worker, daemon=True)
    worker_thread.start()
    server = HTTPServer(('127.0.0.1', args.port), Handler)
    server.timeout = 30
    try: server.serve_forever()
    finally:
        stop.set(); server.server_close(); worker_thread.join(timeout=1); journal_lock.close()


if __name__ == '__main__': main()

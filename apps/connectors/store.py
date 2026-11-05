# SPDX-License-Identifier: AGPL-3.0-only
"""Durable single-account journal. Back up this file alongside the credentials."""
from contextlib import contextmanager
import hashlib
import json
import sqlite3


class Store:
    def __init__(self, path, installation):
        self.path, self.installation = path, installation
        with self.connect() as db:
            db.executescript('''
                CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY AUTOINCREMENT, source TEXT UNIQUE NOT NULL, payload TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS threads(id TEXT PRIMARY KEY, payload TEXT NOT NULL);
                CREATE TABLE IF NOT EXISTS outbound(ref TEXT PRIMARY KEY, digest TEXT NOT NULL, payload TEXT NOT NULL, state TEXT NOT NULL);
            ''')
            previous = db.execute("SELECT value FROM settings WHERE key='installation'").fetchone()
            if previous and previous[0] != installation:
                raise ValueError('Installation identity cannot change for an existing journal')
            db.execute("INSERT OR IGNORE INTO settings VALUES('installation',?)", (installation,))
            # A crash after submission can have an external effect. Never resend blindly.
            for ref, payload in db.execute("SELECT ref,payload FROM outbound WHERE state='sending'").fetchall():
                self._status(db, ref, json.loads(payload), 'unknown')

    @contextmanager
    def connect(self):
        db = sqlite3.connect(self.path, timeout=10)
        db.execute('PRAGMA journal_mode=WAL')
        db.execute('PRAGMA synchronous=FULL')
        try:
            with db:
                yield db
        finally:
            db.close()

    def get(self, key, default=None):
        with self.connect() as db:
            row = db.execute('SELECT value FROM settings WHERE key=?', (key,)).fetchone()
            return json.loads(row[0]) if row else default

    def set(self, key, value):
        with self.connect() as db:
            db.execute('INSERT OR REPLACE INTO settings VALUES(?,?)', (key, json.dumps(value)))

    def incoming(self, source, event, thread):
        with self.connect() as db:
            if db.execute('SELECT 1 FROM events WHERE source=?', (source,)).fetchone():
                return
            db.execute('INSERT INTO events(source,payload) VALUES(?,?)', (source, json.dumps(event)))
            db.execute('INSERT OR REPLACE INTO threads VALUES(?,?)', (event['thread_id'], json.dumps(thread)))

    def thread(self, key):
        with self.connect() as db:
            row = db.execute('SELECT payload FROM threads WHERE id=?', (key,)).fetchone()
            if not row:
                raise ValueError('Unknown conversation')
            return json.loads(row[0])

    def events(self, since, limit):
        with self.connect() as db:
            rows = db.execute('SELECT id,payload FROM events WHERE id>? ORDER BY id LIMIT ?', (since, limit)).fetchall()
        result, size = [], 0
        for id, payload in rows:
            item = dict(json.loads(payload), id=id, version=1, installation_id=self.installation)
            length = len(json.dumps(item).encode())
            if size + length > 1_900_000: break
            result.append(item); size += length
        return result

    def enqueue(self, value):
        if (value.get('version') != 1 or value.get('channel') != 'email'
                or not isinstance(value.get('client_ref'), str) or not 1 <= len(value['client_ref']) <= 200
                or not isinstance(value.get('body'), str) or not 1 <= len(value['body']) <= 20000
                or not isinstance(value.get('subject'), str) or len(value['subject']) > 500):
            raise ValueError('Invalid text email')
        ref = value['client_ref']
        payload = json.dumps(value, sort_keys=True, separators=(',', ':'))
        digest = hashlib.sha256(payload.encode()).hexdigest()
        self.thread(value.get('thread_id'))
        with self.connect() as db:
            row = db.execute('SELECT digest FROM outbound WHERE ref=?', (ref,)).fetchone()
            if row and row[0] != digest:
                raise ValueError('Idempotency key has different content')
            db.execute("INSERT OR IGNORE INTO outbound VALUES(?,?,?,'queued')", (ref, digest, payload))
        return {'message_id': 'out:' + ref, 'idempotent': True}

    def _status(self, db, ref, value, state):
        event = dict(kind='message.status', message_id='out:' + ref,
                     client_ref=ref, thread_id=value['thread_id'], channel='email', state=state, metadata={})
        db.execute('UPDATE outbound SET state=? WHERE ref=?', (state, ref))
        db.execute('INSERT OR IGNORE INTO events(source,payload) VALUES(?,?)', ('out:' + ref + ':' + state, json.dumps(event)))

    def drain(self, provider):
        with self.connect() as db:
            db.execute('BEGIN IMMEDIATE')
            row = db.execute("SELECT ref,payload FROM outbound WHERE state='queued' ORDER BY rowid LIMIT 1").fetchone()
            if not row:
                return
            ref, payload = row
            db.execute("UPDATE outbound SET state='sending' WHERE ref=?", (ref,))
        value = json.loads(payload)
        try:
            provider.send(value, self.thread(value['thread_id']))
            state = 'sent'  # Provider acceptance, never delivered/read.
        except Exception:
            state = 'unknown'  # Reconcile at provider before a deliberate new send.
        with self.connect() as db:
            self._status(db, ref, value, state)

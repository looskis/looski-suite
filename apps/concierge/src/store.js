// SPDX-License-Identifier: AGPL-3.0-only
import { Database } from 'bun:sqlite';
import { mkdirSync, chmodSync } from 'node:fs';
import { dirname } from 'node:path';
import { randomUUID } from 'node:crypto';

export class Store {
  constructor(path) {
    if (path !== ':memory:')
      mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
    this.db = new Database(path, { create: true, strict: true });
    if (path !== ':memory:') chmodSync(path, 0o600);
    this.db.exec(`PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
      CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, data TEXT NOT NULL, updated INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS requests (session TEXT, id TEXT, fingerprint TEXT NOT NULL, response TEXT, PRIMARY KEY(session,id));
      CREATE TABLE IF NOT EXISTS limits (key TEXT PRIMARY KEY, n INTEGER NOT NULL, expires INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY AUTOINCREMENT, payload TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS outbound (ref TEXT PRIMARY KEY, fingerprint TEXT NOT NULL, response TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS tickets (id TEXT PRIMARY KEY, session TEXT NOT NULL, expires INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS mail_threads (id TEXT PRIMARY KEY, source TEXT UNIQUE NOT NULL, token TEXT UNIQUE NOT NULL, recipient TEXT NOT NULL, subject TEXT NOT NULL, last_message TEXT NOT NULL, created INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS mail_outbox (ref TEXT PRIMARY KEY, fingerprint TEXT NOT NULL, thread TEXT NOT NULL, payload TEXT NOT NULL, state TEXT NOT NULL, provider_id TEXT, error TEXT, created INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS mail_inbound (id TEXT PRIMARY KEY, thread TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS processing (id TEXT PRIMARY KEY, state TEXT NOT NULL);
    `);
  }
  get(id) {
    const row = this.db.query('SELECT data FROM sessions WHERE id=?').get(id);
    return row
      ? JSON.parse(row.data)
      : { id, messages: [], lead: null, cards: [], facts: {}, handoff: false };
  }
  save(session) {
    this.db
      .query(
        'INSERT INTO sessions VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data,updated=excluded.updated',
      )
      .run(session.id, JSON.stringify(session), Date.now());
  }
  consume(key, max, windowMs) {
    return this.db.transaction(() => {
      const now = Date.now();
      this.db.query('DELETE FROM limits WHERE expires<?').run(now);
      const row = this.db.query('SELECT n FROM limits WHERE key=?').get(key);
      if (row?.n >= max) return false;
      this.db
        .query(
          'INSERT INTO limits VALUES(?,1,?) ON CONFLICT(key) DO UPDATE SET n=n+1',
        )
        .run(key, now + windowMs);
      return true;
    })();
  }
  event(session, body, direction = 'inbound', channel = 'web', extra = {}) {
    const payload = {
      version: 1,
      installation_id: 'looski-concierge-v1',
      kind: 'message.received',
      message_id: randomUUID(),
      thread_id: session.id,
      channel,
      sender:
        direction === 'inbound'
          ? session.lead?.email || session.callerPhone || 'Website visitor'
          : 'Looski assistant',
      destination:
        session.channel === 'email'
          ? 'Looski email'
          : session.channel === 'phone'
            ? '+16469921008'
            : 'Looski',
      ...(extra.sender ? { sender: extra.sender } : {}),
      body,
      subject:
        extra.subject ||
        (session.lead?.name
          ? `${session.lead.name} · Looski concierge`
          : session.channel === 'phone'
            ? 'Looski phone conversation'
            : 'Website conversation'),
      state: 'received',
      metadata: {
        direction,
        received_at: new Date().toISOString(),
        ...extra,
        assistant: direction === 'outbound',
        ...(session.channel === 'phone'
          ? { call_id: session.phoneCallId, transport: 'retell' }
          : {}),
      },
    };
    const row = this.db
      .query('INSERT INTO events(payload) VALUES(?) RETURNING id')
      .get(JSON.stringify(payload));
    return { ...payload, id: row.id };
  }
  events(after, limit = 50) {
    return this.db
      .query('SELECT id,payload FROM events WHERE id>? ORDER BY id LIMIT ?')
      .all(after, limit)
      .map(({ id, payload }) => ({ ...JSON.parse(payload), id }));
  }
  ticket(session) {
    this.db.query('DELETE FROM tickets WHERE expires<?').run(Date.now());
    const id = randomUUID();
    this.db
      .query('INSERT INTO tickets VALUES(?,?,?)')
      .run(id, session, Date.now() + 60000);
    return id;
  }
  takeTicket(id) {
    const row = this.db
      .query('DELETE FROM tickets WHERE id=? RETURNING session,expires')
      .get(id);
    return row && row.expires > Date.now() ? row.session : null;
  }
}

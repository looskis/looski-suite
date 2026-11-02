// SPDX-License-Identifier: AGPL-3.0-only
import { test, expect } from 'bun:test';
import { Store } from './store.js';
import {
  mailAuthorized,
  mailThread,
  queueMail,
  receiveMail,
  receiveReceipt,
  drainMail,
  queueBookingMail,
} from './mail.js';
const config = {
  postmarkKey: 'test',
  mailSecret: 'secret',
  mailFrom: 'team@example.com',
  inboundAddress: 'hash@inbound.postmarkapp.com',
  bookingEmail: 'owner@example.com',
};
const setup = () => {
  const store = new Store(':memory:');
  const source = store.get(crypto.randomUUID());
  source.lead = { name: 'Visitor', email: 'visitor@example.com', phone: '' };
  source.facts.need = 'Private AI';
  return {
    store,
    source,
    thread: mailThread(store, config, source, 'visitor@example.com', 'Demo'),
  };
};
test('mail webhooks require the exact private basic credential', () => {
  expect(mailAuthorized(null, 'secret')).toBe(false);
  expect(
    mailAuthorized(
      'Basic ' + Buffer.from('postmark:secret').toString('base64'),
      'secret',
    ),
  ).toBe(true);
  expect(
    mailAuthorized(
      'Basic ' + Buffer.from('postmark:wrong').toString('base64'),
      'secret',
    ),
  ).toBe(false);
});
test('durable send retries never duplicate an ambiguous provider effect', async () => {
  const { store, thread } = setup();
  const ref = crypto.randomUUID();
  queueMail(store, config, thread, ref, 'Hello');
  let sends = 0;
  const fetcher = async () => {
    sends++;
    throw Error('timeout after acceptance');
  };
  await drainMail(store, config, fetcher);
  queueMail(store, config, thread, ref, 'Hello');
  await drainMail(store, config, fetcher);
  expect(sends).toBe(1);
  expect(store.db.query('SELECT state FROM mail_outbox').get().state).toBe(
    'uncertain',
  );
  expect(() => queueMail(store, config, thread, ref, 'Changed')).toThrow();
  receiveReceipt(store, {
    RecordType: 'Delivery',
    MessageID: 'provider-1',
    Metadata: { concierge_ref: ref },
  });
  expect(store.db.query('SELECT state FROM mail_outbox').get().state).toBe(
    'delivered',
  );
});
test('provider acceptance is distinct from delivery and preserved across retry', async () => {
  const { store, thread } = setup();
  queueMail(store, config, thread, 'reference', 'Hello');
  let sends = 0;
  await drainMail(store, config, async () => {
    sends++;
    return Response.json({ ErrorCode: 0, MessageID: 'provider-1' });
  });
  await drainMail(store, config, async () => {
    sends++;
  });
  expect(sends).toBe(1);
  expect(store.db.query('SELECT state FROM mail_outbox').get().state).toBe(
    'accepted',
  );
});
test('inbound capability binds the sender, rejects loops and deduplicates webhooks', () => {
  const { store, thread } = setup();
  const msg = {
    MessageID: 'inbound-1',
    FromFull: { Email: 'visitor@example.com' },
    ToFull: [{ Email: config.inboundAddress }],
    MailboxHash: thread.token,
    TextBody: 'Help with setup',
    Headers: [{ Name: 'Message-ID', Value: '<reply@example.com>' }],
  };
  expect(
    receiveMail(store, config, {
      ...msg,
      FromFull: { Email: 'someone@example.com' },
    }),
  ).toEqual({ ignored: true });
  expect(
    receiveMail(store, config, {
      ...msg,
      Headers: [{ Name: 'Auto-Submitted', Value: 'auto-replied' }],
    }),
  ).toEqual({ ignored: true });
  expect(receiveMail(store, config, msg)).toEqual({ received: true });
  expect(receiveMail(store, config, msg)).toEqual({ duplicate: true });
  const events = store.events(0);
  expect(events).toHaveLength(1);
  expect(events[0].thread_id).toBe(thread.id);
  expect(events[0].destination).toBe('Looski email');
  expect(events[0].metadata.received_at).toBeTruthy();
  queueMail(
    store,
    config,
    store.db.query('SELECT * FROM mail_threads WHERE id=?').get(thread.id),
    'reply',
    'Here is help',
  );
  const mail = JSON.parse(
    store.db.query('SELECT payload FROM mail_outbox').get().payload,
  );
  expect(mail.Headers).toContainEqual({
    Name: 'In-Reply-To',
    Value: '<reply@example.com>',
  });
});
test('booking notifications queue separately for owner and visitor once', () => {
  const { store, source } = setup();
  const booking = { id: crypto.randomUUID(), startsAt: '2026-11-03T21:00:00Z' };
  queueBookingMail(store, config, source, booking);
  queueBookingMail(store, config, source, booking);
  source.facts.need = 'Updated';
  queueBookingMail(store, config, source, booking);
  expect(store.db.query('SELECT count(*) AS n FROM mail_outbox').get().n).toBe(
    2,
  );
  queueBookingMail(store, config, source, booking, true);
  expect(store.db.query('SELECT count(*) AS n FROM mail_outbox').get().n).toBe(
    4,
  );
});

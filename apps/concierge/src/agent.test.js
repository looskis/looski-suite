import { test, expect } from 'bun:test';
import { Store } from './store.js';
import { executeTool, controlIntent, respond } from './agent.js';
import { authorized, validateLead, createApp } from './server.js';
import { randomUUID } from 'node:crypto';
const catalog = [
  {
    name: 'blueski',
    summary: 'Messages',
    install: 'brew install looskis/tap/blueski',
  },
];
test('tool calls cannot execute arbitrary operations or invent catalog entries', () => {
  const s = { cards: [], facts: {} };
  expect(() => executeTool('delete_contacts', {}, s, catalog)).toThrow();
  executeTool(
    'recommend_tools',
    { names: ['malicious-command', 'blueski'] },
    s,
    catalog,
  );
  expect(s.cards).toEqual([{ type: 'tool', tool: catalog[0] }]);
  executeTool('recommend_tools', { names: ['blueski'] }, s, catalog);
  expect(s.cards.length).toBe(1);
  executeTool('show_demo_booking', {}, s, catalog);
  executeTool('show_demo_booking', {}, s, catalog);
  expect(s.cards.filter((c) => c.type === 'booking').length).toBe(1);
  expect(s.booking).toBeUndefined();
});
test('authentication denies missing, mismatched and malformed credentials', () => {
  expect(authorized(null, 'secret')).toBe(false);
  expect(authorized('Bearer wrong', 'secret')).toBe(false);
  expect(authorized('Bearer secret', 'secret')).toBe(true);
});
test('contact capture requires explicit consent and validated identity', () => {
  expect(() =>
    validateLead({ name: 'Name', email: 'a@example.com' }),
  ).toThrow();
  expect(() =>
    validateLead({ name: 'Name', email: 'nope', consent: true }),
  ).toThrow();
  expect(
    validateLead({ name: 'Name', email: 'A@example.com', consent: true }).email,
  ).toBe('a@example.com');
});
test('voice tickets are expiring single-use capabilities', () => {
  const store = new Store(':memory:');
  const ticket = store.ticket('session');
  expect(store.takeTicket(ticket)).toBe('session');
  expect(store.takeTicket(ticket)).toBeNull();
  const expired = store.ticket('session');
  store.db.query('UPDATE tickets SET expires=0 WHERE id=?').run(expired);
  expect(store.takeTicket(expired)).toBeNull();
});
test('rate reservations enforce a durable shared cap', () => {
  const store = new Store(':memory:');
  expect(store.consume('network', 1, 60000)).toBe(true);
  expect(store.consume('network', 1, 60000)).toBe(false);
});
test('stop and human requests have deterministic behavior', async () => {
  expect(controlIntent('STOP')).toBe('stop');
  expect(controlIntent('I want nonstop availability')).toBeNull();
  const s = { cards: [], messages: [], facts: {} };
  const result = await respond({}, s, 'STOP', [], () => {
    throw new Error('Model must not be called');
  });
  expect(result).toContain('paused');
  expect(s.handoff).toBe(true);
  expect(s.stopped).toBe(true);
});
const config = { bridgeKey: 'a'.repeat(64), catalog, dbPath: ':memory:' };
const request = (body) =>
  new Request('http://localhost/v1/concierge', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${config.bridgeKey}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });
test('contact retries create one Suite intake and changed replay is rejected', async () => {
  let calls = 0;
  const app = createApp(config, new Store(':memory:'), {
    capture: async () => {
      calls++;
      return { id: 'intake', contact_id: 'contact' };
    },
  });
  const body = {
    session: randomUUID(),
    requestId: randomUUID(),
    network: 'b'.repeat(64),
    action: 'contact',
    lead: { name: 'Test', email: 'test@example.com', consent: true },
  };
  expect((await app.handle(request(body))).status).toBe(200);
  expect((await app.handle(request(body))).status).toBe(200);
  expect(calls).toBe(1);
  expect(
    (
      await app.handle(
        request({
          ...body,
          lead: { ...body.lead, email: 'changed@example.com' },
        }),
      )
    ).status,
  ).toBe(409);
});
test('booking requires saved contact and an offered slot; capability survives ambiguous outcomes', async () => {
  let calls = 0;
  const store = new Store(':memory:');
  const app = createApp(config, store, {
    book: async () => {
      calls++;
      throw new Error('suite_503');
    },
  });
  const id = randomUUID(),
    body = {
      session: id,
      requestId: randomUUID(),
      network: 'a'.repeat(64),
      action: 'book',
      startsAt: '2026-12-01T15:00:00Z',
    };
  expect((await app.handle(request(body))).status).toBe(400);
  expect(calls).toBe(0);
  const s = store.get(id);
  s.lead = { name: 'Test', email: 'test@example.com' };
  store.save(s);
  expect(
    (await app.handle(request({ ...body, requestId: randomUUID() }))).status,
  ).toBe(400);
  s.offeredSlots = [body.startsAt];
  store.save(s);
  expect((await app.handle(request(body))).status).toBe(503);
  const code = store.get(id).bookingIntent.accessCode;
  expect(code.length).toBe(64);
  expect((await app.handle(request(body))).status).toBe(503);
  expect(store.get(id).bookingIntent.accessCode).toBe(code);
  expect(store.get(id).booking).toBeUndefined();
});
test('public responses never expose contact IDs, identity values or booking capabilities', async () => {
  const store = new Store(':memory:');
  const s = store.get(randomUUID());
  s.lead = { email: 'private@example.com' };
  s.bookingIntent = { accessCode: 'secret' };
  s.contactId = 'private-id';
  store.save(s);
  const app = createApp(config, store, {});
  const result = await (
    await app.handle(
      request({
        session: s.id,
        requestId: randomUUID(),
        network: 'c'.repeat(64),
        action: 'state',
      }),
    )
  ).text();
  expect(result).not.toContain('private@example.com');
  expect(result).not.toContain('secret');
  expect(result).not.toContain('private-id');
});
import { eligibleMessage } from './channels.js';
test('historical imports, groups and missing classification never trigger replies', () => {
  const since = new Date(Date.now() - 60000).toISOString();
  const conversation = { channel: 'imessage' };
  const message = {
    direction: 'inbound',
    created_at: new Date().toISOString(),
    metadata: {
      provider_timestamp: new Date().toISOString(),
      thread_kind: 'direct',
      membership_complete: true,
      classification_conflict: false,
    },
  };
  expect(eligibleMessage(conversation, message, since)).toBe(true);
  expect(
    eligibleMessage(
      conversation,
      {
        ...message,
        metadata: {
          ...message.metadata,
          provider_timestamp: '2020-01-01T00:00:00Z',
        },
      },
      since,
    ),
  ).toBe(false);
  expect(
    eligibleMessage(
      conversation,
      { ...message, metadata: { ...message.metadata, thread_kind: 'group' } },
      since,
    ),
  ).toBe(false);
  expect(
    eligibleMessage(conversation, { ...message, metadata: {} }, since),
  ).toBe(false);
});

test('active voice prevents contact mutation while state stays available', async () => {
  const app = createApp(config, new Store(':memory:'), {});
  const id = randomUUID();
  app.voiceActive.add(id);
  const body = {
    session: id,
    requestId: randomUUID(),
    network: 'a'.repeat(64),
    action: 'contact',
    lead: { name: 'Test', email: 'test@example.com', consent: true },
  };
  expect((await app.handle(request(body))).status).toBe(409);
  expect((await app.handle(request({ ...body, action: 'state' }))).status).toBe(
    200,
  );
});
test('cancellation only acts on the current session booking and supports replay', async () => {
  const store = new Store(':memory:');
  let calls = 0;
  const app = createApp(config, store, {
    cancel: async () => {
      calls++;
    },
  });
  const body = {
    session: randomUUID(),
    requestId: randomUUID(),
    network: 'a'.repeat(64),
    action: 'cancel',
  };
  expect((await app.handle(request(body))).status).toBe(400);
  const s = store.get(body.session);
  s.booking = { id: randomUUID(), startsAt: '2026-12-01T15:00:00Z' };
  s.bookingIntent = { accessCode: 'private' };
  store.save(s);
  expect((await app.handle(request(body))).status).toBe(200);
  expect((await app.handle(request(body))).status).toBe(200);
  expect(calls).toBe(1);
  expect(store.get(body.session).booking).toBeNull();
});

import { startChannels } from './channels.js';
test('a queued STOP suppresses earlier unanswered texts before any model call', async () => {
  const store = new Store(':memory:');
  const id = randomUUID(),
    account = randomUUID(),
    sent = [];
  const metadata = {
    provider_timestamp: new Date().toISOString(),
    thread_kind: 'direct',
    membership_complete: true,
    classification_conflict: false,
  };
  const messages = ['Please help me with AI', 'STOP'].map((body) => ({
    id: randomUUID(),
    direction: 'inbound',
    body,
    metadata,
  }));
  const app = {
    store,
    busy: new Set(),
    suite: {
      workspace: async (path, options) => {
        if (options?.method === 'POST') {
          sent.push(options.body.body);
          return { id: randomUUID() };
        }
        if (path.startsWith('communications/conversations?'))
          return { items: [{ id, account_id: account, channel: 'imessage' }] };
        return { items: messages };
      },
    },
  };
  const stop = startChannels(app, {
    autoReplyAccounts: [account],
    channelsSince: new Date(Date.now() - 60000).toISOString(),
  });
  for (let i = 0; i < 20 && !sent.length; i++) await Bun.sleep(5);
  stop();
  expect(sent.length).toBe(1);
  expect(sent[0]).toContain('paused');
  expect(store.get(id).stopped).toBe(true);
  expect(store.get(id).messages[0].text).toBe('STOP');
});

test('an empty final provider turn preserves useful verified tool results', async () => {
  const session = { cards: [], facts: {}, messages: [], lead: null };
  let calls = 0;
  const text = await respond(
    { modelUrl: 'https://example.invalid', model: 'test' },
    session,
    'Help me use Messages',
    catalog,
    async () =>
      Response.json({
        choices: [
          {
            message:
              ++calls === 1
                ? {
                    role: 'assistant',
                    tool_calls: [
                      {
                        id: 'tool1',
                        function: {
                          name: 'recommend_tools',
                          arguments: JSON.stringify({ names: ['blueski'] }),
                        },
                      },
                    ],
                  }
                : { role: 'assistant', content: null },
          },
        ],
      }),
  );
  expect(text).toContain('blueski');
  expect(text).toContain('setup cards');
  expect(session.cards.length).toBe(1);
});

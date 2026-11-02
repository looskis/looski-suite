import { test, expect } from 'bun:test';
import { Store } from './store.js';
import {
  phoneSecretMatches,
  verifyPhoneCall,
  phoneSessionId,
  runPhoneTool,
  confirmsPending,
  commitPhoneConfirmation,
} from './retell.js';
const config = {
  retellKey: 'test',
  retellAgent: 'agent-test',
  retellNumber: '+16469921008',
};
test('Retell admission requires secret plus a live call for the configured agent and number', async () => {
  expect(phoneSecretMatches('wrong', 'secret')).toBe(false);
  expect(phoneSecretMatches('secret', 'secret')).toBe(true);
  const call = {
    call_id: 'call_test_12345',
    agent_id: 'agent-test',
    call_status: 'ongoing',
    call_type: 'phone_call',
    direction: 'inbound',
    to_number: config.retellNumber,
  };
  const verify = (overrides) =>
    verifyPhoneCall(config, call.call_id, async () =>
      Response.json({ ...call, ...overrides }),
    );
  expect(await verify({})).not.toBeNull();
  expect(await verify({ agent_id: 'other-agent' })).toBeNull();
  expect(await verify({ direction: 'outbound' })).toBeNull();
  expect(await verify({ to_number: '+15555550100' })).toBeNull();
  expect(await verify({ call_status: 'ended' })).toBeNull();
  expect(phoneSessionId(call.call_id)).toBe(phoneSessionId(call.call_id));
});
test('phone contact capture needs a later explicit yes after the complete readback', async () => {
  const store = new Store(':memory:');
  let saved = 0;
  const app = {
      store,
      suite: {
        capture: async () => {
          saved++;
          return { id: 'intake', contact_id: 'contact' };
        },
      },
    },
    id = crypto.randomUUID();
  const result = await runPhoneTool(
    app,
    'propose_contact',
    { name: 'Test Person', email: 'test@example.com', phone: '+15555550100' },
    id,
    1,
  );
  expect(saved).toBe(0);
  const s = store.get(id);
  expect(
    confirmsPending(
      s,
      [
        { role: 'agent', content: 'Would you like some help?' },
        { role: 'user', content: 'yes' },
      ],
      2,
    ),
  ).toBe(false);
  expect(
    confirmsPending(
      s,
      [
        { role: 'agent', content: result.speak },
        { role: 'user', content: 'yes but change the email' },
      ],
      2,
    ),
  ).toBe(false);
  expect(
    confirmsPending(
      s,
      [
        { role: 'agent', content: result.speak },
        { role: 'user', content: 'yes' },
      ],
      1,
    ),
  ).toBe(false);
  expect(
    confirmsPending(
      s,
      [
        { role: 'agent', content: result.speak },
        { role: 'user', content: 'Yes, please.' },
      ],
      2,
    ),
  ).toBe(true);
  await commitPhoneConfirmation(app, id);
  expect(saved).toBe(1);
  expect(store.get(id).lead.email).toBe('test@example.com');
  expect(store.get(id).phonePending).toBeUndefined();
});
test('phone booking only proposes verified slots and persists its retry capability', async () => {
  const store = new Store(':memory:');
  let bookings = 0;
  const id = crypto.randomUUID();
  const app = {
    store,
    suite: {
      book: async () => {
        bookings++;
        throw Error('suite_503');
      },
    },
  };
  expect(
    (
      await runPhoneTool(
        app,
        'propose_booking',
        { starts_at: '2026-12-01T14:00:00Z' },
        id,
        1,
      )
    ).error,
  ).toBeDefined();
  const s = store.get(id);
  s.lead = { name: 'Test', email: 'test@example.com' };
  s.offeredSlots = ['2026-12-01T14:00:00Z'];
  store.save(s);
  expect(
    (
      await runPhoneTool(
        app,
        'propose_booking',
        { starts_at: '2026-12-01T18:00:00Z' },
        id,
        1,
      )
    ).error,
  ).toBeDefined();
  const proposal = await runPhoneTool(
    app,
    'propose_booking',
    { starts_at: s.offeredSlots[0] },
    id,
    2,
  );
  expect(proposal.speak).toContain('Eastern');
  expect(bookings).toBe(0);
  await expect(commitPhoneConfirmation(app, id)).rejects.toThrow();
  const first = store.get(id).bookingIntent;
  await expect(commitPhoneConfirmation(app, id)).rejects.toThrow();
  expect(store.get(id).bookingIntent).toEqual(first);
  expect(first.accessCode.length).toBe(64);
  expect(store.get(id).booking).toBeUndefined();
});

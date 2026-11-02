// SPDX-License-Identifier: AGPL-3.0-only
import { readFileSync } from 'node:fs';
import { createHash, timingSafeEqual, randomUUID } from 'node:crypto';
import { Store } from './store.js';
import { Suite } from './suite.js';
import { respond } from './agent.js';
import { voiceHandlers } from './voice.js';
import { startChannels } from './channels.js';
import {
  mailAuthorized,
  receiveMail,
  receiveReceipt,
  queueMail,
  startMail,
} from './mail.js';
import { blueskiBridge } from './bridge.js';
import {
  phoneHandlers,
  phoneSecretMatches,
  verifyPhoneCall,
} from './retell.js';

export const digest = (text) => createHash('sha256').update(text).digest('hex');
export const authorized = (supplied, expected) =>
  !!expected &&
  typeof supplied === 'string' &&
  timingSafeEqual(
    Buffer.from(digest(supplied)),
    Buffer.from(digest(`Bearer ${expected}`)),
  );
const uuid = (value) =>
  typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f-]{27}$/i.test(value);
const reply = (body, status = 200) =>
  Response.json(body, { status, headers: { 'Cache-Control': 'no-store' } });
export function validateLead(value) {
  if (
    !value ||
    typeof value.name !== 'string' ||
    !value.name.trim() ||
    value.name.length > 120 ||
    typeof value.email !== 'string' ||
    value.email.length > 254 ||
    !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value.email) ||
    (value.phone &&
      (typeof value.phone !== 'string' ||
        !/^\+?[\d\s().-]{7,30}$/.test(value.phone))) ||
    (value.company &&
      (typeof value.company !== 'string' || value.company.length > 160)) ||
    value.consent !== true
  )
    throw new Error('invalid_details');
  return {
    name: value.name.trim(),
    email: value.email.trim().toLowerCase(),
    phone: value.phone?.trim() || '',
    company: value.company?.trim() || '',
    consentAt: new Date().toISOString(),
    consentVersion: 'concierge-response-v1',
  };
}
export function publicSession(session) {
  return {
    messages: session.messages.slice(-40),
    cards: session.cards,
    hasContact: !!session.lead,
    handoff: session.handoff,
    booking: session.booking
      ? {
          startsAt: session.booking.startsAt,
          name: 'Looski private AI demo',
          id: session.booking.id,
        }
      : null,
  };
}
export function createApp(
  config,
  store = new Store(config.dbPath),
  suite = new Suite(config),
) {
  const busy = new Set();
  const voiceActive = new Set();
  async function handle(req) {
    const url = new URL(req.url);
    if (url.pathname === '/healthz')
      return reply({ status: 'ok', service: 'looski-suite-concierge' });
    if (url.pathname === '/mail/inbound' || url.pathname === '/mail/receipt') {
      if (
        req.method !== 'POST' ||
        !mailAuthorized(req.headers.get('authorization'), config.mailSecret)
      )
        return reply({ error: 'Unauthorized' }, 401);
      try {
        const message = JSON.parse(await readBounded(req, 12000000));
        if (url.pathname === '/mail/inbound')
          receiveMail(store, config, message);
        else receiveReceipt(store, message);
        return reply({ ok: true });
      } catch {
        return reply({ error: 'Invalid mail' }, 400);
      }
    }
    if (!authorized(req.headers.get('authorization'), config.bridgeKey))
      return reply({ error: 'Unauthorized' }, 401);
    if (url.pathname.startsWith('/blueski/')) return blueskiBridge(req, config);
    if (url.pathname === '/v1/capabilities')
      return reply({
        version: 1,
        installation_id: 'looski-concierge-v1',
        idempotent_send: true,
      });
    if (url.pathname === '/v1/events') {
      const since = Number(url.searchParams.get('since') || 0);
      if (!Number.isSafeInteger(since) || since < 0)
        return reply({ error: 'Invalid cursor' }, 400);
      return reply(store.events(since));
    }
    if (
      req.method !== 'POST' ||
      !['/v1/concierge', '/v1/messages'].includes(url.pathname)
    )
      return reply({ error: 'Not found' }, 404);
    if (!req.headers.get('content-type')?.startsWith('application/json'))
      return reply({ error: 'JSON required' }, 415);
    let body;
    try {
      body = JSON.parse(await readBounded(req, 14000));
    } catch {
      return reply({ error: 'Invalid request' }, 400);
    }
    if (url.pathname === '/v1/messages') {
      // Human replies from the Suite inbox are delivered back to the browser.
      if (
        !uuid(body.thread_id) ||
        !uuid(body.client_ref) ||
        typeof body.body !== 'string' ||
        body.body.length > 6000
      )
        return reply({ error: 'Invalid message' }, 400);
      const fingerprint = digest(JSON.stringify(body));
      const prior = store.db
        .query('SELECT * FROM outbound WHERE ref=?')
        .get(body.client_ref);
      if (prior)
        return prior.fingerprint === fingerprint
          ? reply(JSON.parse(prior.response))
          : reply({ error: 'Conflict' }, 409);
      if (busy.has(body.thread_id)) return reply({ error: 'Busy' }, 503);
      const session = store.get(body.thread_id);
      if (
        !store.db
          .query('SELECT id FROM sessions WHERE id=?')
          .get(body.thread_id)
      )
        return reply({ error: 'Not found' }, 404);
      if (session.channel === 'email') {
        const thread = store.db
          .query('SELECT * FROM mail_threads WHERE id=?')
          .get(session.id);
        if (!thread || body.channel !== 'email')
          return reply({ error: 'Invalid email thread' }, 409);
        try {
          return reply(
            queueMail(store, config, thread, body.client_ref, body.body),
          );
        } catch {
          return reply({ error: 'Email conflict or unavailable' }, 409);
        }
      }
      if (session.channel === 'phone')
        return reply(
          {
            error:
              'Phone conversations require a callback; text delivery is unavailable.',
          },
          409,
        );
      session.handoff = true;
      session.messages.push({
        id: body.client_ref,
        role: 'assistant',
        text: body.body,
        human: true,
      });
      const result = { message_id: body.client_ref, idempotent: true };
      store.db.transaction(() => {
        store.save(session);
        store.db
          .query('INSERT INTO outbound VALUES(?,?,?)')
          .run(body.client_ref, fingerprint, JSON.stringify(result));
      })();
      return reply(result);
    }
    const { session: id, action, requestId, network } = body;
    if (
      !uuid(id) ||
      !uuid(requestId) ||
      typeof network !== 'string' ||
      !/^[a-f0-9]{64}$/.test(network)
    )
      return reply({ error: 'Invalid session' }, 400);
    if (
      ![
        'state',
        'message',
        'contact',
        'slots',
        'book',
        'cancel',
        'handoff',
        'resume',
        'voice',
      ].includes(action)
    )
      return reply({ error: 'Unknown action' }, 400);
    if (action === 'state' && !store.consume(`state:${network}`, 180, 60000))
      return reply({ error: 'Please wait a minute before trying again.' }, 429);
    if (action === 'state')
      return reply({
        ...publicSession(store.get(id)),
        voiceAvailable: config.voiceEnabled === true,
      });
    if (!store.consume(`minute:${network}`, 40, 60000))
      return reply({ error: 'Please wait a minute before trying again.' }, 429);
    if (voiceActive.has(id))
      return reply(
        { error: 'End voice before continuing in chat or saving details.' },
        409,
      );
    if (busy.has(id))
      return reply({ error: 'Please wait for the current reply.' }, 409);
    const fingerprint = digest(
      JSON.stringify({
        action,
        message: body.message,
        lead: body.lead,
        date: body.date,
        startsAt: body.startsAt,
      }),
    );
    const prior = store.db
      .query('SELECT * FROM requests WHERE session=? AND id=?')
      .get(id, requestId);
    if (prior && prior.fingerprint !== fingerprint)
      return reply({ error: 'Request changed. Please try again.' }, 409);
    if (prior?.response) return reply(JSON.parse(prior.response));
    busy.add(id);
    const session = store.get(id);
    try {
      store.db
        .query('INSERT OR IGNORE INTO requests VALUES(?,?,?,NULL)')
        .run(id, requestId, fingerprint);
      let result;
      if (action === 'message') {
        if (
          typeof body.message !== 'string' ||
          !body.message.trim() ||
          body.message.length > 4000
        )
          return reply(
            { error: 'Write a message of up to 4,000 characters.' },
            400,
          );
        if (session.handoff) {
          session.messages.push({
            id: requestId,
            role: 'user',
            text: body.message.trim(),
          });
          store.event(session, body.message.trim());
          store.save(session);
          result = { ...publicSession(session), awaitingHuman: true };
        } else {
          if (
            !store.consume(`day:${network}`, 150, 86400000) ||
            !store.consume(`session:${id}`, 80, 86400000) ||
            !store.consume('daily:global', config.dailyLimit || 2500, 86400000)
          )
            return reply(
              {
                error:
                  'You’ve reached the conversation allowance for today. You can still leave your details or book a demo.',
              },
              429,
            );
          const text = await respond(
            config,
            session,
            body.message.trim(),
            config.catalog,
          );
          session.messages.push(
            { id: requestId, role: 'user', text: body.message.trim() },
            { id: randomUUID(), role: 'assistant', text },
          );
          store.event(session, body.message.trim());
          store.event(session, text, 'outbound');
          store.save(session);
          result = publicSession(session);
        }
      } else if (action === 'contact') {
        const lead = validateLead(body.lead);
        const intake = await suite.capture(session, lead, requestId);
        session.lead = lead;
        session.intakeId = intake.id;
        session.notifyLead = true;
        session.contactId = intake.contact_id;
        session.cards = session.cards.filter((c) => c.type !== 'contact');
        session.messages.push({
          id: randomUUID(),
          role: 'assistant',
          text: 'Thanks—your details are saved with Looski. What would you like to tackle next?',
        });
        store.event(
          session,
          `Contact details saved for ${lead.name}. Response consent recorded.`,
          'outbound',
        );
        store.save(session);
        result = publicSession(session);
      } else if (action === 'slots') {
        if (
          typeof body.date !== 'string' ||
          !/^\d{4}-\d{2}-\d{2}$/.test(body.date)
        )
          return reply({ error: 'Choose a valid date.' }, 400);
        result = await suite.slots(body.date);
        session.offeredSlots = result.slots?.map((s) => s.starts_at) || [];
        store.save(session);
      } else if (action === 'book') {
        if (!session.lead)
          return reply(
            { error: 'Save your name and email before booking.' },
            400,
          );
        if (session.booking) return reply(publicSession(session));
        if (!session.offeredSlots?.includes(body.startsAt))
          return reply(
            { error: 'Choose one of the available appointment times.' },
            400,
          );
        // Booking intent and capability survive a crash or an ambiguous Suite response.
        session.bookingIntent ||= {
          startsAt: body.startsAt,
          key: requestId,
          accessCode: (await import('node:crypto'))
            .randomBytes(32)
            .toString('hex'),
        };
        store.save(session);
        const receipt = await suite.book(session, body.startsAt, requestId);
        session.booking = { id: receipt.id, startsAt: body.startsAt };
        (session.mailEvents ||= []).push({
          booking: session.booking,
          cancelled: false,
        });
        session.cards = session.cards.filter((c) => c.type !== 'booking');
        session.messages.push({
          id: randomUUID(),
          role: 'assistant',
          text: 'Your demo is booked. Your confirmed appointment is shown below. We’ll use the contact details you provided.',
        });
        store.event(
          session,
          `Demo booked: ${body.startsAt}. Appointment ${receipt.id}.`,
          'outbound',
        );
        store.save(session);
        result = publicSession(session);
      } else if (action === 'cancel') {
        if (!session.booking)
          return reply({ error: 'No active booking.' }, 400);
        await suite.cancel(session, requestId);
        store.event(
          session,
          `Demo cancelled: ${session.booking.id}.`,
          'outbound',
        );
        (session.mailEvents ||= []).push({
          booking: session.booking,
          cancelled: true,
        });
        session.booking = null;
        delete session.bookingIntent;
        session.messages.push({
          id: randomUUID(),
          role: 'assistant',
          text: 'Your demo is cancelled. You can choose another time whenever you are ready.',
        });
        store.save(session);
        result = publicSession(session);
      } else if (action === 'handoff') {
        if (!session.lead)
          return reply(
            { error: 'Save your contact details so our team can respond.' },
            400,
          );
        session.handoff = true;
        session.cards = [];
        session.messages.push({
          id: randomUUID(),
          role: 'assistant',
          text: 'Your request is in our team’s inbox. You can keep adding context here. For something urgent, call +1 (646) 992-1008.',
        });
        store.event(
          session,
          `HUMAN FOLLOW-UP REQUESTED: ${session.lead.name}. ${session.facts.need || ''}`,
        );
        store.save(session);
        result = publicSession(session);
      } else if (action === 'resume') {
        session.handoff = false;
        session.stopped = false;
        store.save(session);
        result = publicSession(session);
      } else if (action === 'voice') {
        if (!config.voiceEnabled)
          return reply(
            {
              error:
                'Voice is not available yet. You can chat here or call Looski.',
            },
            503,
          );
        if (
          !store.consume(`voice:${network}`, 3, 86400000) ||
          !store.consume(
            'voice:global',
            config.dailyVoiceLimit || 100,
            86400000,
          )
        )
          return reply(
            { error: 'Voice allowance reached. Continue in chat.' },
            429,
          );
        result = { ticket: store.ticket(id), url: config.voiceUrl };
      }
      store.db
        .query('UPDATE requests SET response=? WHERE session=? AND id=?')
        .run(JSON.stringify(result), id, requestId);
      return reply(result);
    } catch (error) {
      console.error(
        JSON.stringify({
          event: 'concierge_error',
          action,
          code: /^([a-z]+_\d+|invalid_details|booking_in_progress|model_empty|model_tool_limit)$/.test(
            error.message,
          )
            ? error.message
            : 'unavailable',
        }),
      );
      if (action === 'book' && error.message === 'suite_409') {
        delete session.bookingIntent;
        store.save(session);
      }
      const invalid = error.message === 'invalid_details';
      return reply(
        {
          error: invalid
            ? 'Check your name, email and phone, and confirm we may respond.'
            : action === 'book'
              ? 'That booking could not be confirmed. Refresh the available times and try again.'
              : 'That didn’t go through. Your conversation is still here—please try again.',
        },
        invalid ? 400 : 503,
      );
    } finally {
      busy.delete(id);
    }
  }
  return { handle, store, suite, busy, voiceActive };
}
export async function readBounded(req, limit) {
  const reader = req.body?.getReader();
  if (!reader) throw new Error('empty');
  let size = 0;
  const chunks = [];
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.length;
    if (size > limit) {
      await reader.cancel();
      throw new Error('too_large');
    }
    chunks.push(value);
  }
  return Buffer.concat(chunks).toString('utf8');
}
if (import.meta.main) {
  const config = JSON.parse(
    readFileSync(process.env.LOOSKI_CONCIERGE_CONFIG, 'utf8'),
  );
  if (!config.bridgeKey || config.bridgeKey.length < 32 || !config.modelKey)
    throw new Error('Missing private concierge configuration');
  const app = createApp(config);
  const browserVoice = voiceHandlers(app, config);
  const telephone = phoneHandlers(app, config);
  const transport = (ws) =>
    ws.data.channel === 'phone' ? telephone : browserVoice;
  Bun.serve({
    hostname: '127.0.0.1',
    port: config.port || 58900,
    async fetch(req, server) {
      const path = new URL(req.url).pathname;
      // Override any account-wide Retell webhook without accepting business
      // mutations from it. Live transcripts arrive on the authenticated socket.
      if (path.startsWith('/retell-events/')) {
        if (
          req.method !== 'POST' ||
          !phoneSecretMatches(
            path.slice('/retell-events/'.length),
            config.retellSecret,
          )
        )
          return reply({ error: 'Unauthorized' }, 401);
        return new Response(null, { status: 204 });
      }
      if (path.startsWith('/retell/')) {
        const parts = path.split('/');
        if (
          parts.length !== 4 ||
          !phoneSecretMatches(parts[2], config.retellSecret)
        )
          return reply({ error: 'Unauthorized' }, 401);
        try {
          const call = await verifyPhoneCall(config, parts[3]);
          if (!call) return reply({ error: 'Unauthorized call' }, 403);
          if (server.upgrade(req, { data: { channel: 'phone', call } })) return;
        } catch {
          return reply({ error: 'Call unavailable' }, 503);
        }
        return reply({ error: 'WebSocket required' }, 400);
      }
      if (new URL(req.url).pathname === '/voice') {
        if (!config.origins?.includes(req.headers.get('origin')))
          return reply({ error: 'Unauthorized origin' }, 403);
        if (server.upgrade(req, { data: {} })) return;
        return reply({ error: 'WebSocket required' }, 400);
      }
      return app.handle(req);
    },
    websocket: {
      maxPayloadLength: 131072,
      open: (ws) => transport(ws).open(ws),
      message: (ws, data) => transport(ws).message(ws, data),
      close: (ws) => transport(ws).close(ws),
    },
    maxRequestBodySize: 12000000,
    idleTimeout: 60,
  });
  startChannels(app, config);
  startMail(app, config);
  console.log('Looski Suite concierge listening on loopback');
}

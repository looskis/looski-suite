// SPDX-License-Identifier: AGPL-3.0-only
import {
  createHash,
  randomBytes,
  randomUUID,
  timingSafeEqual,
} from 'node:crypto';
const hash = (value) => createHash('sha256').update(value).digest('hex');
const address = (value) =>
  typeof value === 'string' &&
  value.length <= 254 &&
  /^[^\s@,<>]+@[^\s@,<>]+\.[^\s@,<>]+$/.test(value);
export function mailAuthorized(value, secret) {
  return (
    !!secret &&
    timingSafeEqual(
      Buffer.from(hash(value || '')),
      Buffer.from(
        hash(`Basic ${Buffer.from(`postmark:${secret}`).toString('base64')}`),
      ),
    )
  );
}
export function mailThread(
  store,
  config,
  source,
  recipient,
  subject,
  owner = false,
) {
  if (!address(recipient)) throw Error('invalid_recipient');
  const key = `${source.id}:${recipient.toLowerCase()}`;
  const prior = store.db
    .query('SELECT * FROM mail_threads WHERE source=?')
    .get(key);
  if (prior) return prior;
  const id = randomUUID(),
    token = randomBytes(24).toString('hex');
  const session = store.get(id);
  session.channel = 'email';
  session.handoff = owner;
  session.facts = { ...source.facts };
  session.lead =
    source.lead && source.lead.email.toLowerCase() === recipient.toLowerCase()
      ? source.lead
      : null;
  session.contactId = session.lead ? source.contactId : null;
  store.db.transaction(() => {
    store.save(session);
    store.db
      .query('INSERT INTO mail_threads VALUES(?,?,?,?,?,?,?)')
      .run(
        id,
        key,
        token,
        recipient.toLowerCase(),
        subject.slice(0, 180),
        '',
        Date.now(),
      );
  })();
  return store.db.query('SELECT * FROM mail_threads WHERE id=?').get(id);
}
export function queueMail(store, config, thread, ref, text) {
  if (!config.postmarkKey) throw Error('mail_unavailable');
  const fingerprint = hash(JSON.stringify([thread.id, text]));
  const prior = store.db
    .query('SELECT * FROM mail_outbox WHERE ref=?')
    .get(ref);
  if (prior) {
    if (prior.fingerprint !== fingerprint) throw Error('mail_conflict');
    return { message_id: ref, idempotent: true };
  }
  const [local, domain] = config.inboundAddress.split('@');
  const payload = {
    From: `Looski <${config.mailFrom}>`,
    To: thread.recipient,
    ReplyTo: `${local}+${thread.token}@${domain}`,
    Subject: thread.subject,
    TextBody: text,
    MessageStream: config.mailStream || 'outbound',
    TrackOpens: false,
    TrackLinks: 'None',
    Tag: 'looski-concierge',
    Metadata: { concierge_ref: ref },
    Headers: [
      { Name: 'Auto-Submitted', Value: 'auto-generated' },
      ...(thread.last_message
        ? [
            { Name: 'In-Reply-To', Value: thread.last_message },
            { Name: 'References', Value: thread.last_message },
          ]
        : []),
    ],
  };
  store.db
    .query('INSERT INTO mail_outbox VALUES(?,?,?,?,?,?,?,?)')
    .run(
      ref,
      fingerprint,
      thread.id,
      JSON.stringify(payload),
      'queued',
      null,
      null,
      Date.now(),
    );
  return { message_id: ref, idempotent: true };
}
export function queueBookingMail(
  store,
  config,
  session,
  booking,
  cancelled = false,
) {
  if (!config.postmarkKey || !session.lead || !booking) return;
  const state = cancelled ? 'cancelled' : 'confirmed';
  const time = new Intl.DateTimeFormat('en-US', {
    timeZone: 'America/New_York',
    dateStyle: 'full',
    timeStyle: 'short',
  }).format(new Date(booking.startsAt));
  const subject = `Looski demo ${state} · ${time} Eastern`;
  const common = `Your 30-minute Looski private AI demo is ${state}.\n\n${time} Eastern time\nAppointment: ${booking.id}\n\n${cancelled ? 'Choose another time at https://www.loo.ski/#demo.' : 'We will use the contact details you provided to coordinate the demo. No meeting link has been issued yet. You can cancel from the browser where you booked, or reply here for help.'}`;
  for (const recipient of new Set([
    session.lead.email.toLowerCase(),
    config.bookingEmail.toLowerCase(),
  ])) {
    const ref = hash(`${booking.id}:${state}:${recipient}`);
    if (store.db.query('SELECT ref FROM mail_outbox WHERE ref=?').get(ref))
      continue;
    const owner = recipient === config.bookingEmail.toLowerCase();
    const thread = mailThread(
      store,
      config,
      session,
      recipient,
      subject,
      owner,
    );
    // Each booking keeps the current subject while retaining the reply capability.
    thread.subject = subject;
    store.db
      .query('UPDATE mail_threads SET subject=? WHERE id=?')
      .run(subject, thread.id);
    const body = owner
      ? `Looski demo ${state}\n\n${time} Eastern\nName: ${session.lead.name}\nEmail: ${session.lead.email}\nPhone: ${session.lead.phone || 'Not provided'}\nNeed: ${session.facts.need || 'Not provided'}\nAppointment: ${booking.id}\n\nReview and follow up in https://suite.loo.ski.\nNo meeting link has been issued yet.`
      : `${common}\n\nLooski · https://www.loo.ski\nReplies are handled by our AI assistant in Looski Suite. Reply HUMAN to pause automation and ask the team.`;
    queueMail(store, config, thread, ref, body);
  }
}
export function receiveMail(store, config, message) {
  const messageId = message?.MessageID;
  if (typeof messageId !== 'string' || messageId.length > 300)
    throw Error('invalid_mail');
  if (store.db.query('SELECT id FROM mail_inbound WHERE id=?').get(messageId))
    return { duplicate: true };
  const sender = message.FromFull?.Email?.toLowerCase();
  const headers = Array.isArray(message.Headers) ? message.Headers : [];
  const header = (name) =>
    headers.find((h) => h.Name?.toLowerCase() === name)?.Value || '';
  const suppressed =
    !address(sender) ||
    (sender === config.mailFrom.toLowerCase() && !message.MailboxHash) ||
    (!!header('auto-submitted') &&
      header('auto-submitted').toLowerCase() !== 'no') ||
    !!header('list-id') ||
    /bulk|junk|list/i.test(header('precedence')) ||
    (Array.isArray(message.CcFull) && message.CcFull.length > 0) ||
    (message.ToFull?.length || 0) !== 1;
  const token = message.MailboxHash || message.ToFull?.[0]?.MailboxHash || '';
  let thread =
    token &&
    store.db.query('SELECT * FROM mail_threads WHERE token=?').get(token);
  if (!token || !thread || thread.recipient !== sender)
    return { ignored: true };
  if (suppressed) return { ignored: true };
  const text = (message.StrippedTextReply || message.TextBody || '')
    .trim()
    .slice(0, 6000);
  if (!text) return { ignored: true };
  if (
    !store.consume(`mail:${hash(sender)}`, 30, 86400000) ||
    !store.consume('mail:inbound', 500, 86400000)
  )
    return { ignored: true };
  store.db.transaction(() => {
    const session = store.get(thread.id);
    // Provider IDs are not necessarily RFC Message-IDs; preserve the actual header.
    const reference = header('message-id');
    if (/^<[^\r\n<>]{1,900}>$/.test(reference))
      store.db
        .query('UPDATE mail_threads SET last_message=? WHERE id=?')
        .run(reference, thread.id);
    store.event(session, text, 'inbound', 'email', {
      sender,
      subject: thread.subject,
      provider_message_id: messageId,
    });
    store.db
      .query('INSERT INTO mail_inbound VALUES(?,?)')
      .run(messageId, thread.id);
  })();
  return { received: true };
}
export function receiveReceipt(store, message) {
  const ref = message.Metadata?.concierge_ref;
  if (typeof ref !== 'string') return;
  const row = store.db.query('SELECT * FROM mail_outbox WHERE ref=?').get(ref);
  if (!row || (row.provider_id && row.provider_id !== message.MessageID))
    return;
  const state = {
    Delivery: 'delivered',
    Bounce: 'failed',
    SpamComplaint: 'failed',
  }[message.RecordType];
  if (!state) return;
  if (row.state === state && row.provider_id === message.MessageID) return;
  if (row.state === 'delivered' && state !== 'delivered') return;
  store.db
    .query(
      'UPDATE mail_outbox SET state=?,provider_id=?,error=NULL WHERE ref=?',
    )
    .run(state, message.MessageID, ref);
  mailStatus(store, row, state);
}
function mailStatus(store, row, state) {
  const payload = {
    version: 1,
    installation_id: 'looski-concierge-v1',
    kind: 'message.status',
    message_id: row.ref,
    thread_id: row.thread,
    client_ref: row.ref,
    channel: 'email',
    destination: 'Looski email',
    state,
  };
  store.db
    .query('INSERT INTO events(payload) VALUES(?)')
    .run(JSON.stringify(payload));
}
export async function drainMail(store, config, fetcher = fetch) {
  // dispatching survives a crash as an uncertain outcome. Never blindly replay.
  const rows = store.db
    .query(
      "SELECT * FROM mail_outbox WHERE state='queued' ORDER BY created LIMIT 10",
    )
    .all();
  for (const row of rows) {
    if (
      !store.db
        .query(
          "UPDATE mail_outbox SET state='dispatching' WHERE ref=? AND state='queued'",
        )
        .run(row.ref).changes
    )
      continue;
    try {
      const response = await fetcher('https://api.postmarkapp.com/email', {
        method: 'POST',
        redirect: 'error',
        signal: AbortSignal.timeout(12000),
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
          'X-Postmark-Server-Token': config.postmarkKey,
        },
        body: row.payload,
      });
      const result = await response.json();
      if (!response.ok || result.ErrorCode) {
        const state =
          response.status >= 400 && response.status < 500
            ? 'failed'
            : 'uncertain';
        store.db
          .query(
            "UPDATE mail_outbox SET state=?,error=? WHERE ref=? AND state='dispatching'",
          )
          .run(
            state,
            `postmark_${response.status}_${result.ErrorCode || 0}`,
            row.ref,
          );
        mailStatus(store, row, state === 'failed' ? 'failed' : 'unknown');
      } else {
        if (!result.MessageID) throw Error('missing_receipt');
        store.db
          .query(
            "UPDATE mail_outbox SET state='accepted',provider_id=? WHERE ref=? AND state='dispatching'",
          )
          .run(result.MessageID, row.ref);
        mailStatus(store, row, 'accepted');
      }
    } catch {
      store.db
        .query(
          "UPDATE mail_outbox SET state='uncertain',error='transport' WHERE ref=? AND state='dispatching'",
        )
        .run(row.ref);
      mailStatus(store, row, 'unknown');
    }
  }
}
export function queueLeadAlert(store, config, session) {
  const ref = hash(`lead:${session.id}:${session.intakeId}`);
  if (store.db.query('SELECT ref FROM mail_outbox WHERE ref=?').get(ref))
    return;
  const thread = mailThread(
    store,
    config,
    session,
    config.bookingEmail,
    'New Looski conversation',
    true,
  );
  queueMail(
    store,
    config,
    thread,
    ref,
    `New Looski lead

Name: ${session.lead.name}
Email: ${session.lead.email}
Phone: ${session.lead.phone || 'Not provided'}
Need: ${session.facts.need || 'Not provided'}

Review the conversation and follow up in https://suite.loo.ski. Contact permission was explicitly confirmed. This is an automatic notification.`,
  );
}
export function startMail(app, config) {
  if (!config.postmarkKey) return;
  app.store.db
    .query(
      "UPDATE mail_outbox SET state='uncertain',error='interrupted' WHERE state='dispatching'",
    )
    .run();
  let running = false;
  const poll = async () => {
    if (running) return;
    running = true;
    try {
      for (const row of app.store.db
        .query(
          "SELECT data FROM sessions WHERE json_extract(data,'$.mailEvents') IS NOT NULL",
        )
        .all()) {
        const session = JSON.parse(row.data);
        for (const event of session.mailEvents)
          queueBookingMail(
            app.store,
            config,
            session,
            event.booking,
            event.cancelled,
          );
      }
      for (const row of app.store.db
        .query(
          "SELECT data FROM sessions WHERE json_extract(data,'$.notifyLead')=1",
        )
        .all()) {
        const session = JSON.parse(row.data);
        if (session.lead) queueLeadAlert(app.store, config, session);
      }
      await drainMail(app.store, config);
    } catch {
      console.error(JSON.stringify({ event: 'mail_queue_unavailable' }));
    } finally {
      running = false;
    }
  };
  const timer = setInterval(poll, 5000);
  void poll();
  return () => clearInterval(timer);
}

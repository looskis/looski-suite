// SPDX-License-Identifier: AGPL-3.0-only
import { respond, controlIntent } from './agent.js';

export function eligibleMessage(conversation, message, since) {
  const blueski = ['imessage', 'sms', 'rcs'].includes(conversation.channel);
  const timestamp = blueski
    ? message.metadata?.provider_timestamp
    : message.metadata?.received_at;
  return (
    message.direction === 'inbound' &&
    Number.isFinite(Date.parse(timestamp)) &&
    Date.parse(timestamp) >= Date.parse(since) &&
    Date.parse(timestamp) <= Date.now() + 60000 &&
    (!blueski ||
      (message.metadata.thread_kind === 'direct' &&
        message.metadata.membership_complete === true &&
        message.metadata.classification_conflict === false))
  );
}

// Opt-in account IDs only. No historical or ambiguous/group messages get an AI
// reply. Durable request keys prevent duplicate sends after a process restart.
export function startChannels(app, config) {
  if (!config.websiteAccount && !config.autoReplyAccounts?.length) return;
  let running = false;
  async function poll() {
    if (running) return;
    running = true;
    try {
      let after = '';
      do {
        const page = await app.suite.workspace(
          `communications/conversations?limit=100${after ? `&after=${after}` : ''}`,
        );
        for (const conversation of page.items) {
          if (
            [config.websiteAccount, config.phoneAccount].includes(
              conversation.account_id,
            )
          ) {
            const visitor = app.store.get(conversation.provider_thread_id);
            if (visitor.contactId && !conversation.contact_id) {
              await app.suite.workspace(
                `communications/conversations/${conversation.id}`,
                {
                  method: 'PATCH',
                  key: `concierge-contact-${conversation.id}-${conversation.version}`,
                  body: {
                    expected_version: conversation.version,
                    contact_id: visitor.contactId,
                    archived: false,
                  },
                },
              );
            }
            continue;
          }
          if (!config.autoReplyAccounts?.includes(conversation.account_id))
            continue;
          const id =
            conversation.channel === 'email'
              ? conversation.provider_thread_id
              : conversation.id;
          if (app.busy.has(id)) continue;
          app.busy.add(id);
          try {
            const session = app.store.get(id);
            const page = await app.suite.workspace(
              `communications/conversations/${conversation.id}/messages?limit=100${session.messageCursor ? `&after=${session.messageCursor}` : ''}`,
            );
            // Observe a queued STOP/HUMAN before generating replies to earlier
            // messages in the same imported page.
            const staffReply = page.items.some(
              (m) =>
                m.direction === 'outbound' &&
                Date.parse(m.created_at) >= Date.parse(config.channelsSince) &&
                !app.store.db
                  .query(
                    "SELECT 1 FROM processing WHERE json_extract(state,'$.outboundId')=?",
                  )
                  .get(m.id),
            );
            if (staffReply) session.handoff = true;
            const lastControl = page.items
              .filter(
                (m) =>
                  eligibleMessage(conversation, m, config.channelsSince) &&
                  controlIntent(m.body),
              )
              .at(-1);
            if (lastControl) {
              session.handoff = true;
              session.stopped = controlIntent(lastControl.body) === 'stop';
              app.store.save(session);
            }
            for (const message of page.items) {
              const eligible = eligibleMessage(
                conversation,
                message,
                config.channelsSince,
              );
              if (
                message.direction === 'outbound' &&
                Date.parse(message.created_at) >=
                  Date.parse(config.channelsSince)
              ) {
                const automatic = app.store.db
                  .query(
                    "SELECT 1 FROM processing WHERE json_extract(state,'$.outboundId')=?",
                  )
                  .get(message.id);
                if (!automatic) session.handoff = true;
              }
              const control = controlIntent(message.body);
              if (!eligible || (session.handoff && !control)) {
                session.messageCursor = message.id;
                app.store.save(session);
                continue;
              }
              // Pause on staff replies. Agent-authored reply IDs have a processing record.
              if (session.stopped && !control) {
                session.messageCursor = message.id;
                app.store.save(session);
                continue;
              }
              if (
                !app.store.consume(`channel:${id}`, 30, 86400000) ||
                !app.store.consume(
                  'daily:global',
                  config.dailyLimit || 2500,
                  86400000,
                )
              )
                break;
              const saved = app.store.db
                .query('SELECT state FROM processing WHERE id=?')
                .get(message.id);
              let text;
              if (saved) text = JSON.parse(saved.state).text;
              else {
                session.channel = conversation.channel;
                text = await respond(
                  config,
                  session,
                  message.body,
                  config.catalog,
                );
                app.store.db
                  .query('INSERT INTO processing VALUES(?,?)')
                  .run(message.id, JSON.stringify({ text }));
                app.store.save(session);
              }
              const outbound = await app.suite.workspace(
                `communications/conversations/${conversation.id}/messages`,
                {
                  method: 'POST',
                  key: `concierge-reply-${message.id}`,
                  body: { body: text },
                },
              );
              app.store.db
                .query('UPDATE processing SET state=? WHERE id=?')
                .run(
                  JSON.stringify({ text, outboundId: outbound.id }),
                  message.id,
                );
              session.messages.push(
                { id: message.id, role: 'user', text: message.body },
                { id: `reply-${message.id}`, role: 'assistant', text },
              );
              session.messageCursor = message.id;
              app.store.save(session);
            }
          } finally {
            app.busy.delete(id);
          }
        }
        after = page.next_cursor || '';
      } while (after);
    } catch {
      console.error(JSON.stringify({ event: 'channel_poll_unavailable' }));
    } finally {
      running = false;
    }
  }
  const timer = setInterval(poll, 5000);
  void poll();
  return () => clearInterval(timer);
}

// SPDX-License-Identifier: AGPL-3.0-only
import { randomUUID } from 'node:crypto';
import { createGateway } from '@ai-sdk/gateway';
import { tools, instructions, executeTool, controlIntent } from './agent.js';

// The Mac mini owns the session and tools. The browser can submit only audio;
// neither Gateway credentials nor provider-issued tokens leave this process.
export function voiceHandlers(app, config) {
  const active = app.voiceActive;
  const gateway = createGateway({ apiKey: config.modelKey });
  const modelId = config.voiceModel || 'google/gemini-3.8-live';
  const model = gateway.experimental_realtime(modelId);
  const send = (socket, event) =>
    socket.send(JSON.stringify(model.serializeClientEvent(event)));
  const notify = (ws, data) => {
    if (ws.readyState === 1) ws.send(JSON.stringify(data));
  };
  function saveTranscript(ws, role, text) {
    if (!text?.trim()) return;
    const s = app.store.get(ws.data.sessionId);
    s.messages.push({
      id: randomUUID(),
      role,
      text: text.trim().slice(0, 12000),
    });
    app.store.event(
      s,
      text.trim().slice(0, 12000),
      role === 'user' ? 'inbound' : 'outbound',
      'voice',
    );
    const control = role === 'user' && controlIntent(text);
    if (control) {
      s.handoff = true;
      s.stopped = control === 'stop';
    }
    app.store.save(s);
    notify(ws, { type: 'update' });
    if (control) ws.close();
  }
  return {
    maxPayloadLength: 16000,
    open(ws) {
      ws.data.authTimer = setTimeout(
        () => ws.close(1008, 'Authentication required'),
        5000,
      );
    },
    async message(ws, data) {
      if (!ws.data.sessionId) {
        // Prevent overlapping ticket redemption while token minting is pending.
        if (ws.data.authenticating) {
          ws.close(1008);
          return;
        }
        ws.data.authenticating = true;
        let ticket;
        try {
          ticket = JSON.parse(String(data)).ticket;
        } catch {
          ws.close(1008);
          return;
        }
        if (typeof ticket !== 'string' || ticket.length > 64) {
          ws.close(1008);
          return;
        }
        const id = app.store.takeTicket(ticket);
        if (!id || active.has(id) || app.busy.has(id)) {
          ws.close(1008);
          return;
        }
        const session = app.store.get(id);
        if (session.handoff) {
          notify(ws, {
            type: 'error',
            message: 'Resume AI chat before starting voice.',
          });
          ws.close();
          return;
        }
        active.add(id);
        ws.data.sessionId = id;
        clearTimeout(ws.data.authTimer);
        ws.data.deadline = setTimeout(() => {
          notify(ws, {
            type: 'error',
            message:
              'This voice session has ended. Your conversation is saved; continue in chat.',
          });
          ws.close();
        }, 300000);
        try {
          const token = await gateway.experimental_realtime.getToken({
            model: modelId,
            expiresAfterSeconds: 60,
          });
          if (ws.readyState !== 1) return;
          const connection = model.getWebSocketConfig(token);
          const upstream = new WebSocket(connection.url, connection.protocols);
          ws.data.upstream = upstream;
          ws.data.bytes = 0;
          ws.data.started = Date.now();
          upstream.onopen = () => {
            send(upstream, {
              type: 'session-update',
              config: {
                instructions:
                  instructions(config.catalog, session) +
                  '\nYou are speaking by voice. Use brief natural speech. Tool cards appear in the browser. Never read code aloud. Introduce yourself as an AI assistant. Recent conversation: ' +
                  JSON.stringify(session.messages.slice(-8)),
                outputModalities: ['audio'],
                inputAudioFormat: { type: 'audio/pcm', rate: 16000 },
                outputAudioFormat: { type: 'audio/pcm', rate: 24000 },
                inputAudioTranscription: {},
                outputAudioTranscription: {},
                turnDetection: { type: 'server-vad' },
                tools: tools.map((t) => ({ type: 'function', ...t })),
              },
            });
          };
          let transcript = '';
          const completed = new Set();
          upstream.onmessage = async (event) => {
            if (app.store.get(id).handoff) {
              ws.close();
              return;
            }
            try {
              const raw =
                typeof event.data === 'string'
                  ? event.data
                  : await event.data.text();
              const packet = model.parseServerEvent(JSON.parse(raw));
              if (
                [
                  'session-created',
                  'session-updated',
                  'session-started',
                ].includes(packet.type) &&
                !ws.data.ready
              ) {
                ws.data.ready = true;
                notify(ws, { type: 'ready' });
              }
              if (
                packet.type === 'speech-started' ||
                (packet.type === 'custom' &&
                  packet.raw?.voiceActivity?.type === 'ACTIVITY_START')
              )
                notify(ws, { type: 'interrupt' });
              if (['audio-delta', 'audio-chunk'].includes(packet.type))
                ws.send(Buffer.from(packet.delta, 'base64'));
              if (packet.type === 'input-transcription-completed')
                saveTranscript(ws, 'user', packet.transcript);
              if (packet.type === 'audio-transcript-delta')
                transcript += packet.delta;
              if (
                packet.type === 'audio-transcript-done' ||
                (packet.type === 'custom' &&
                  packet.raw?.serverContent?.turnComplete)
              ) {
                saveTranscript(
                  ws,
                  'assistant',
                  packet.transcript || transcript,
                );
                transcript = '';
              }
              if (
                packet.type === 'function-call-arguments-done' &&
                !completed.has(packet.callId)
              ) {
                completed.add(packet.callId);
                const current = app.store.get(id);
                let result;
                try {
                  result = executeTool(
                    packet.name,
                    JSON.parse(packet.arguments),
                    current,
                    config.catalog,
                  );
                } catch {
                  result = { error: 'Unavailable action' };
                }
                app.store.save(current);
                notify(ws, { type: 'update' });
                send(upstream, {
                  type: 'conversation-item-create',
                  item: {
                    type: 'function-call-output',
                    callId: packet.callId,
                    name: packet.name,
                    output: JSON.stringify(result),
                  },
                });
              }
              if (packet.type === 'error') {
                console.error(
                  JSON.stringify({
                    event: 'voice_provider_error',
                    code: packet.code || 'unavailable',
                  }),
                );
                notify(ws, {
                  type: 'error',
                  message: 'Voice was interrupted. Continue in chat.',
                });
                ws.close();
              }
            } catch {
              notify(ws, {
                type: 'error',
                message: 'Voice was interrupted. Continue in chat.',
              });
              ws.close();
            }
          };
          upstream.onerror = () => {
            notify(ws, {
              type: 'error',
              message: 'Voice could not connect. Continue in chat.',
            });
            ws.close();
          };
          upstream.onclose = () => ws.close();
        } catch {
          notify(ws, {
            type: 'error',
            message: 'Voice could not connect. Please try again.',
          });
          ws.close();
        }
        return;
      }
      if (
        typeof data === 'string' ||
        !ws.data.ready ||
        data.length % 2 !== 0 ||
        data.length > 8192
      )
        return;
      ws.data.bytes += data.length;
      if (ws.data.bytes > ((Date.now() - ws.data.started) / 1000 + 3) * 36000) {
        ws.close(1008);
        return;
      }
      if (
        ws.data.upstream?.readyState === WebSocket.OPEN &&
        ws.data.upstream.bufferedAmount < 128000
      )
        send(ws.data.upstream, {
          type: 'input-audio-append',
          audio: Buffer.from(data).toString('base64'),
        });
    },
    close(ws) {
      clearTimeout(ws.data.deadline);
      clearTimeout(ws.data.authTimer);
      ws.data.upstream?.close();
      if (ws.data.sessionId) active.delete(ws.data.sessionId);
    },
  };
}

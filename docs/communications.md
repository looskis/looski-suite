# Communications

Communications provides a Rust-owned inbox for connected messaging accounts. The first slice supports incoming Blueski messages, replies to existing direct/group threads, delivery/read receipts, contact links, contact timelines, and a versioned HTTP connector for providers implemented in any language. API, web, CLI and MCP use the same business operations.

The built-in Blueski adapter is implemented against the [public Blueski contract](https://github.com/looskis/blueski/blob/main/openapi.json). HTTP v1 has an email-shaped acceptance fixture (subjects, email thread IDs and headers in metadata). It does **not** include a Gmail OAuth connector, IMAP synchronization or a standalone email delivery service. Calls, outbound attachments, new-conversation composition, assignment queues, are outside the core inbox slice. The optional concierge service below supplies bounded automatic replies. Existing Helpdesk tickets remain independent; connecting tickets to these conversations is a later integration.

## Connect Blueski

Run `make setup` to create `.local/<project>/communications.json` with mode 0600. API and worker receive that file as a Compose secret; credentials are never accepted in browser forms or returned through the API. Configure a connection in that JSON array:

```json
[
  {
    "key": "customer-messages",
    "workspace_id": "YOUR-WORKSPACE-UUID",
    "provider": "blueski",
    "installation_id": "BLUESKI-INSTALLATION-ID",
    "endpoint": "https://YOUR-AUTHENTICATED-BLUESKI-ENDPOINT",
    "api_token": "YOUR-BLUESKI-BEARER-TOKEN",
    "webhook_secret": "YOUR-UNIQUE-AT-LEAST-32-CHARACTER-SECRET",
    "destinations": ["YOUR-RECEIVING-ADDRESS"]
  }
]
```

Use Blueski's `/healthz` to obtain its installation identity. Endpoints must use HTTPS, except loopback HTTP for a native local deployment. Redirects are refused. A Linux container's `127.0.0.1` is not the host Mac: a container installation needs an authenticated reachable Blueski endpoint or a separately configured bridge. Keep Blueski running on macOS; its Messages integration cannot run inside the Linux worker container.

`destinations` is an exact-match inbound receiving-address allowlist. If it is nonempty, events without an allowed destination are not stored. An empty list explicitly allows all incoming traffic from that connected installation. Matching preserves provider formatting. Allocate one connector key per source installation; do not bind the same source to different workspaces unless deliberately provisioning independently filtered connections. Provider and installation identities are pinned when connected; use a new key/account if the underlying installation changes. Do not repoint an account to a fresh installation with reset cursors.

Recreate API and worker after changing the secret file. `make up` applies the normal migration/startup workflow. Open **Communications**, enable the application, and connect the configured key. The account belongs to the connecting member and starts private. Sharing permits other members with workspace-scoped Communications grants to access it; a workspace-wide grant alone does not reveal a private account. Only the owner can change sharing. Pausing an account stops receiving and sending, while its stored history remains readable.

After creating the account, configure a Blueski webhook to:

```text
https://YOUR-LOOSKI-ORIGIN/api/v1/connectors/communications/ACCOUNT-UUID/events
```

Use the same `webhook_secret` in Blueski. Looski verifies `X-Blueski-Signature` as HMAC-SHA256 over the exact request bytes and commits the event and message before returning 204. Webhooks are optional: the worker polls replayable events, one eligible account every ten seconds. **Refresh messages** also imports one page. Large backlogs are imported in bounded pages. A failed page leaves its cursor unchanged and surfaces a connection error.

## Connector v1

Native Rust integrations implement `communications::adapter::ChannelAdapter`. The bundled `http_v1` adapter supports an external connector with this contract:

- `GET /v1/capabilities` returns `{version:1,installation_id,idempotent_send:true}`. The worker verifies identity and durable retry support before every send. Blueski must report version 0.2.0 or later from `/healthz`.
- `GET /v1/events?since=<integer>&limit=50` returns an ascending JSON array. IDs are positive, monotonic within the pinned `installation_id`; return at most the requested limit. Retain events for replay and never reuse an ID with changed content.
- `POST /v1/messages` accepts `{version:1,client_ref,thread_id,channel,body,subject}`. It must durably deduplicate `client_ref`, reject changed requests under that key, and return `{message_id,idempotent:true}` only after committing the outbound operation. Keep idempotency records across restarts. Looski retries the same key after ambiguous network outcomes.
- Both calls receive `Authorization: Bearer <api_token>`. Responses are bounded to 2 MB, requests time out after five seconds, and redirects are disabled.
- Optional callbacks use the same Looski event endpoint with `X-Connector-Signature` containing hex HMAC-SHA256 of the raw body (an optional `sha256=` prefix is accepted). A callback never advances the polling cursor; replay recovers out-of-order gaps.

Example incoming event:

```json
{
  "version": 1,
  "installation_id": "mail-gateway-1",
  "id": 42,
  "kind": "message.received",
  "message_id": "<message@example.com>",
  "thread_id": "provider-thread-9",
  "channel": "email",
  "sender": "customer@example.com",
  "destination": "support@example.com",
  "body": "Can you help with my order?",
  "subject": "Order question",
  "client_ref": null,
  "state": "received",
  "metadata": {"in_reply_to": "<earlier@example.com>"}
}
```

For receipt events, use `kind: "message.status"`, the original Looski `client_ref`, and `state` of `accepted`, `sent`, `delivered`, `read`, `failed`, or `unknown`. `message.queued`, `message.sent` and `message.failed` are also supported event kinds. Receipts can only update an outbound message in the same connected account. Preserve the provider message identity once assigned. Do not fabricate delivered/read receipts when a provider cannot confirm them.

Set `provider: "http_v1"` and an explicit `capabilities` object in the operator file, for example:

```json
{
  "channels": ["email"],
  "send_text": true,
  "subjects": true,
  "delivery_receipts": true,
  "read_receipts": false,
  "attachments": false,
  "calls": false
}
```

The current contract supports text channels. Adding native calling or attachments requires corresponding core operations and UI support before advertising those capabilities. Provider-specific context remains in message metadata; adapters must not put credentials there. Email connectors must preserve native threading and reply headers themselves. The generic core does not infer email threads from subjects or merge conversations by contact address.

## Reliability and access

Inbound delivery is deduplicated by account/event cursor and by account/provider message ID. The durable event journal retains normalized payloads. Polling and webhooks serialize with account/module mutations through the existing workspace lock. Current account-owner access is checked before unattended ingestion. No inbound event creates or merges a contact automatically.

Outbound intent, job and audit commit together. A worker rechecks the original staff identity's current read/send permissions, account access, account enabled state and conversation archive state immediately before sending. Blueski receives the Looski message UUID as `client_ref`. An accepted send remains `accepted` until provider receipts arrive; Messages.app accepting AppleScript is not proof of delivery. Receipt cursors prevent older updates from regressing newer state. Delivered/read states are not regressed by late failures. Blocked or exhausted jobs appear in message status and can be handled through the existing Jobs administration.

Disabling Communications blocks its pending jobs; re-enabling does not automatically resume them. Private accounts remain private in list/detail views, exports, contact pages and CRM timelines. Contact links require independent Contacts authority, follow contact merges, and protect linked records against permanent deletion. Conversation links do not grant access.

Backups include the new PostgreSQL schema and the private connector file. Recovery pauses connected accounts in addition to quarantining pending jobs, so operators must reconcile external effects and deliberately reconnect/resume after restoration. No automatic message/event deletion or retention purge runs in this slice.

## Interfaces and verification

Staff HTTP routes live under `/api/v1/workspaces/{workspace_id}/communications`: `accounts`, account update/sync, `conversations`, conversation detail/update and `conversations/{id}/messages` read/send. Mutations other than sync require `Idempotency-Key`; metadata updates require `expected_version`. CLI equivalents are under `looski communications`; mutation bodies are JSON on stdin. MCP tools use the `communications_` prefix. Business exports include `communication_conversations` and parent-scoped `communication_messages`, with the same access checks.

Run `cargo test --workspace --locked`, `make check`, and `scripts/communications_smoke.py` against a dedicated `*-identity-check` installation. The integration script clones that fixture into a disposable database, applies the new migration, and uses native API/worker processes with local fake Blueski/email connectors; it never contacts a real provider. Set `COMMUNICATIONS_BROWSER=1` with `pnpm` available to include rendered inbox, reply and contact-page checks. Real Blueski/macOS delivery still requires a configured installation and a deliberately selected test recipient.

## Website concierge service

`apps/concierge` is an optional Bun service owned by this Suite source tree. It supplies a Gemini conversation and browser-voice transport, uses Suite HTTP operations for Contacts and Scheduling, and exposes an HTTP v1 connector so staff can review and reply to website conversations in Communications. It runs on macOS beside Blueski, outside the Linux containers. Install workspace dependencies with the pinned pnpm version, then run `LOOSKI_CONCIERGE_CONFIG=/absolute/private/config.json bun apps/concierge/src/server.js` under a supervisor. Use Bun 1.3.6 or later.

The private JSON configuration needs `port` (58900), `dbPath`, `bridgeKey` (at least 32 random characters), `modelKey` (Vercel AI Gateway), `modelUrl` (`https://ai-gateway.vercel.sh/v1/chat/completions`), `model` (`google/gemini-3.1-flash-lite`), `suiteUrl`, `workspace`, `sessionFile` (Suite CLI session with access/refresh token and millisecond `expires_at`) or `tokenFile`, and `catalog` (approved objects with name, summary, text, install/download). The website catalog is maintained in `looski-v2/apps/web/lib/open-source.ts`; copy its structured entries into private runtime configuration when publishing catalog changes. Keep all private files mode 0600 in a mode 0700 directory. Session refresh updates the file atomically.

For voice set `voiceEnabled`, `voiceModel` (`google/gemini-3.8-live`), the public `voiceUrl` (`wss://…/voice`), and an exact `origins` list. The gateway's experimental realtime protocol is pinned by the package lock. Use server VAD without provider-specific silence options; verify a full audio response and tool result when updating that SDK. Credentials and provider tokens remain server-side. The browser receives a single-use 60-second ticket. Each session lasts at most five minutes. The implementation stores transcripts, not raw audio. `dailyVoiceLimit` defaults to 100 sessions; `dailyLimit` defaults to 2,500 text responses, with lower network/session limits.

Connect an `http_v1` account with installation `looski-concierge-v1`, endpoint at the service origin, its bridge key as `api_token`, destination `Looski`, channels `web` and `voice`, and text/subject capabilities. Set the resulting `websiteAccount` UUID in concierge configuration. `/v1/events` is a persistent replay journal. Transcript entries preserve their speaker in metadata and sender; assistant transcript imports are not delivery receipts. Staff sends use durable client references and immediately pause automation. The service links conversations to contacts resolved by explicit intake; it does not merge contacts from inferred identity. Facts captured later remain in the linked conversation history.

Enable Scheduling, configure an appointment type and its CRM follow-up mapping, and set `bookingType` to its public key. Only offered slots can be booked. Contact consent and booking confirmation are separate explicit actions. The service persists booking intent/capability before sending so retries survive ambiguous failures. Cancellation uses the private capability on behalf of the owning browser session. No email confirmation is claimed without a configured delivery path.

For Blueski container access, configure its Suite connector endpoint as `https://CONCIERGE-ORIGIN/blueski` using the bridge key. The service proxies only `/healthz`, `/events` and `/messages` to loopback Blueski. Configure `blueskiUrl`/`blueskiToken` if needed. Keep the destination allowlist scoped to the business number. Legacy excluded events may advance the replay cursor but are never stored. A missing timestamp or group classification cannot authorize an AI reply.

Automatic channel replies require an explicit `autoReplyAccounts` array and an ISO `channelsSince` cutoff. Disable the previous consumer before activation. Only incoming messages received after that provider timestamp are eligible; importing a historical message now does not make it new. STOP/HUMAN are deterministic. Staff replies pause the conversation. Reply intent and Suite client references survive restarts; sending still goes through the Suite outbox and its permission checks. Do not enable this against a personal/all-destinations account. Empty or absent `autoReplyAccounts` means no unattended external replies.

Back up the concierge SQLite database with SQLite's online backup mechanism and protect it together with private configuration, Suite credentials and the ordinary installation backup. Never delete event IDs, capabilities, or outbound deduplication records during retries. Run `bun run test:concierge`, normal Suite checks, and real browser audio/contact/booking smoke tests before release. Calling-provider and mailbox connectors need their own credentials and integration checks; do not advertise them as enabled merely because the generic adapter supports an email-shaped event.

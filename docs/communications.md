# Communications

Communications provides a Rust-owned inbox for connected messaging accounts. The first slice supports incoming Blueski messages, replies to existing direct/group threads, delivery/read receipts, contact links, contact timelines, and a versioned HTTP connector for providers implemented in any language. API, web, CLI and MCP use the same business operations.

The built-in Blueski adapter is implemented against the [public Blueski contract](https://github.com/looskis/blueski/blob/main/openapi.json). HTTP v1 has an email-shaped acceptance fixture (subjects, email thread IDs and headers in metadata). It does **not** include a Gmail OAuth connector, IMAP synchronization or a standalone email delivery service. Calls, outbound attachments, new-conversation composition, assignment queues, and automatic agent replies are outside this slice. Existing Helpdesk tickets remain independent; connecting tickets to these conversations is a later integration.

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

# Email and calendar connectors

The connector in `apps/connectors` implements the existing Communications HTTP v1 contract, plus a small scheduling HTTP contract. It uses Python 3.11+ standard-library modules and one private SQLite journal per mailbox. Gmail and IMAP share normalized events and Suite's existing private inbox, permissions, contact links, jobs, CLI and MCP. Provider credentials never enter browser forms or Suite API responses.

## Google setup

Enable Gmail and Google Calendar APIs on the OAuth project. A dedicated Google **web application** OAuth client must allow `http://localhost:58911/callback`. Keep its JSON private. Existing Suite sign-in OAuth scopes/callbacks are independent. Run:

```sh
python3 apps/connectors/authorize_google.py --client /private/google-client.json --output /private/kevin-mail/config.json --email kevin@loo.ski --import-after 2026/10/01
python3 apps/connectors/server.py --config /private/kevin-mail/config.json --port 58910
```

The first command opens a local callback listener and prints the Google authorization URL. It uses state and PKCE, checks the granted scopes and exact mailbox, writes a mode-0600 config and never sends email. Required scopes are Gmail read-only, Gmail send and Calendar events. User consent and any organization's OAuth policy must permit them. Initial import uses the explicit date boundary; subsequent polling uses Gmail history IDs, with a full bounded rescan when Google expires the cursor. Cursors advance only after all pages commit. Gmail messages already in Suite are not deleted when removed at the provider. Labels, read/unread and folder mutations are not synchronized in this slice.

For IMAP, use a separate private config with `provider: "imap"`, `address`, `installation_id`, random `api_token`, `username`, `password`, `imap_host`, `imap_port` (993), `folder` (INBOX), `smtp_host`, `smtp_port` (465), and optional `smtp_username`/`smtp_password`. This first adapter uses implicit TLS and app passwords; OAuth IMAP/STARTTLS are future provider capabilities. Polling uses UIDVALIDITY/UID and read-only BODY.PEEK, so importing never marks mail read. Only the configured folder is imported. Configured folders/identity must not be repointed under an existing installation ID.

Expose the loopback connector through a private authenticated HTTPS proxy reachable by the Suite containers. All endpoints require its random bearer token. Containers cannot reach a host process through their own localhost. Use a service manager to restart the process; only one process may own a mailbox journal. Back up the private config and SQLite database together using SQLite's backup API (or stop the service before copying its database/WAL). Restoring the connector requires reconciling queued/unknown sends before restarting delivery. Suite's normal backup includes the connector reference files, not this external journal.

The existing concierge service can host this authenticated relay through `apps/connectors/proxy.mjs`. Its private config accepts `connectors: {"kevin-google": {"url": "http://127.0.0.1:58910", "token": "CONNECTOR-TOKEN"}}`. The external endpoint is then `https://concierge.loo.ski/connectors/kevin-google`, authenticated with the concierge bridge bearer; only the versioned connector routes are forwarded, using the connector's separate local token. This relay is prepared but needs the authorized connector configuration and a concierge service restart before use. Never expose the local connector or operator config directly.

Add an `http_v1` account to the installation's `communications.json`, with matching installation ID/endpoint/token, a distinct webhook secret, exact destination allowlist, and capabilities `channels: ["email"]`, `send_text: true`, `subjects: true`, with `delivery_receipts`, `read_receipts`, `attachments` and `calls` false. Connect that key in Communications. The account starts private. Replies target the original sender only, preserve native reply headers/Gmail thread ID, and do not infer threads from subject. HTML-only messages show a provider-view placeholder; attachments remain in the provider. This is text correspondence, not a full email-client replacement.

Outgoing operations are durably deduplicated by client reference. Changed replay payloads are rejected. A transport error or crash after claiming a send becomes `unknown`, never an automatic resend. Inspect the provider before deliberately sending again. Successful send means provider acceptance, not delivery/read. No messages are sent merely by connecting an account. `/v1/health` reports readiness/provider failure without leaking credentials or message data.

## Scheduling and Google Calendar

The embed is `/book/<public UUID>/embed`; each appointment type includes copyable iframe markup in Scheduling. Full-page bookings keep their frame restriction. Embedded confirmations show the private management code and open management in a new tab, so blocked third-party cookies do not prevent managing a booking. There is no general staff-app framing exemption.

Add a private `calendars.json` array alongside `communications.json`:

```json
[{"workspace_id":"WORKSPACE-UUID","host_id":"MEMBERSHIP-UUID","endpoint":"https://connector.example.com","api_token":"PRIVATE-RANDOM-TOKEN"}]
```

Setup creates an empty array by default; API and worker receive it as a secret. Recreate both after configuring it. Match the booking host's membership ID, not the Auth user ID. Google config selects `calendar: "primary"` plus `conflict_calendars: ["primary"]`; keep the write calendar in the conflict list. Additional readable Google calendars can block time without becoming write targets.

The core calls `POST /v1/calendar/busy` with RFC3339 `start`, `end` and optional appointment UUID `exclude`, expecting `{busy:[{start,end}]}`. Google expands recurring events, includes all-day events in their calendar timezone, excludes transparent/cancelled events and follows pagination. Slot display fetches one day's external busy ranges, applies buffers, then booking/rescheduling rechecks the chosen range. Configured-provider failure returns unavailable rather than offering unchecked slots. Suite's own database serializes competing bookings. Google and Suite cannot share an atomic transaction: an independent Google edit immediately after the final availability check can still race a booking.

Booking creation, rescheduling and cancellation enqueue a `scheduling.calendar` job transactionally. Its current booking version is sent to `POST /v1/calendar/appointments` as `{id,version,state,name,start,end,attendee_name,attendee_email}`. The connector returns `{state:"synced"}` or `superseded`. Stable provider IDs, version/digest checks and conditional ETags recover lost responses without duplicate events or overwriting newer versions. Google events contain attendee details in the host's event description; no attendee invitation or notification is sent by this adapter. Suite manages attendee reminders separately.

Manual changes/deletions in Google cause a conflict on the next outbound update rather than silently overwriting them. They affect availability immediately, but do not automatically cancel/reschedule Suite bookings. Review calendar jobs in Jobs and reconcile the event before retrying. Full two-way appointment edits, push subscriptions, connection management UI and Microsoft/CalDAV adapters are follow-on work; this release provides external availability plus durable Suite-to-Google booking synchronization.

Recovery restores calendar configuration to `calendars-paused.json` and leaves the active file empty. Reconcile the provider and pending jobs before deliberately restoring that connection. Existing appointments from before connection need an explicit backfill; merely adding a connection does not create calendar events for historical bookings.

## Verification and provider references

Run `python3 -m unittest discover -s tests/connectors`, the Rust checks, calendar integration smoke and Svelte check/build. Connector tests cover replay, changed idempotency payloads, ambiguous/crashed sends, cursor failures/history expiry, wrong account rejection, threading, calendar pagination, all-day DST, cancellation retries and external-edit conflicts. Live provider checks still require an authorized account and deliberately selected send recipient; tests do not send real mail.

Contracts follow [Gmail synchronization](https://developers.google.com/workspace/gmail/api/guides/sync), [Gmail threading](https://developers.google.com/workspace/gmail/api/guides/threads), [Calendar event IDs](https://developers.google.com/workspace/calendar/api/v3/reference/events/insert) and [conditional calendar writes](https://developers.google.com/workspace/calendar/api/guides/version-resources). Provider adapters can be replaced without changing Suite's booking or communications records.

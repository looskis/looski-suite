# Helpdesk checkpoint

November 1, 2026; schema version 34. Milestone 6 adds web tickets, assignment, status, immutable public replies and private internal notes. Helpdesk works with every other optional application disabled. The detailed contract is in [the implementation plan](helpdesk-plan.md).

## Customer journey and authority

A staff administrator enables Helpdesk and explicitly opens intake. `/support/new/{public_key}` accepts a subject, requester name and initial message. Names are unverified labels, not identities, and submission never creates or matches a Contact or Auth user.

The web server generates a random 256-bit access code. The API receives it in `X-Ticket-Key`, stores its SHA-256 digest, and returns a ticket ID. The creation response asks the customer to save the ticket address and code; a 30-day HttpOnly, SameSite=Strict cookie is scoped to `/support/t/{ticket_id}` (Secure on HTTPS). Returning from another browser requires entering the saved code. Codes never appear in URLs, audit events, exports or diagnostic fields. API clients must generate a fresh cryptographically random 32-byte lowercase hexadecimal code per ticket. Possession of the code grants access; it is deliberately not proof of a person's identity. Losing it has no email-based recovery path.

Customer read/reply endpoints select only that ticket's subject, status and public messages. They do not return internal notes, staff identities, contact associations, knowledge links, staff version counters or timestamps changed by private work. Message pagination selects public messages before applying its limit; a cursor for a note answers exactly as a nonexistent message. Ordinary staff routes never accept the code as a staff credential.

Staff can permanently revoke a ticket code. Every customer read, message page and mutation checks the current ticket, workspace and enabled application state, including retries. Archive makes customer access unavailable and freezes messages; restore resumes access only if the code was not explicitly revoked. Closing intake stops new submissions, while existing tickets remain accessible. Disabling Helpdesk closes all ordinary staff and public routes, retains all data, and queues no work for replay on re-enable.

Public endpoints use the existing bounded per-client throttle: 30 writes and 180 reads per minute per API process. The gateway sets the client address. This is basic admission control for a single-process installation, not a distributed abuse-prevention service.

## Staff contract

| Permission | Record scope |
| --- | --- |
| `helpdesk.read` | Workspace: all tickets; assigned: only tickets assigned to this membership. Includes internal notes. |
| `helpdesk.manage` | Same scopes; change status, assignee, archive, contact/article links and revoke customer access. |
| `helpdesk.reply` | Same scopes; append a public reply or internal note. |
| `helpdesk.configure` | Workspace only; open/close intake and read its public address. |

Every ticket operation also requires independent read scope. An assigned-only manager cannot change assignment. A manager's reassignment that would remove their own read authority rolls back; reassignment to another member normally needs workspace-wide read/manage. Assignees must be active workspace members. Membership alone grants no Helpdesk permission. The member picker is bounded to 500 active members.

Statuses are `open`, `waiting` and `resolved`. Staff choose any of these; a customer reply reopens waiting or resolved tickets. Private notes and public replies are separate immutable message kinds and cannot be converted, edited or deleted. Bodies are plain text, at most 20,000 characters, rendered as escaped text. Subjects are immutable and limited to 200 characters. The reply form explicitly labels its customer-visible and staff-only choices.

Metadata/link edits use `expected_version`; stale edits return 409. Writes serialize through the established workspace lock, recheck current authority, and commit their changes with metadata-only Helpdesk history and staff audit. Public operations use the immutable Helpdesk event journal with no fictitious staff actor. Creation retries use the code digest and input fingerprint; reply retries use an explicit key. Staff retry records contain hashes and receipts, never message text, and recheck current authority before returning results. A reply retry does not duplicate its message or reopen a ticket a second time.

## Optional connections

Contact linking goes through Contacts' authorized mutation interface and stores a workspace-qualified reference. A merge preserves the original reference and presents the currently permitted survivor. Linking does not grant customer access to the Contact. The CRM contact timeline composes Helpdesk metadata events only under independent Contacts and Helpdesk authority; it hides the source when Helpdesk is disabled. Helpdesk itself does not require CRM.

Knowledge links store document IDs and call `knowledge::previews` under the viewing staff member's current permissions. Disabled or restricted material has no preview. Saving links requires authority over all selected articles. The browser offers contact search and article selection. Knowledge access remains staff-only; sending article contents to a customer is an explicit public reply written by staff, not an automatic consequence of linking it.

## Interfaces, export and recovery

| Capability | Web | HTTP | CLI | MCP |
| --- | --- | --- | --- | --- |
| Staff configuration/directory | Yes | `/workspaces/{id}/helpdesk/settings`, `/directory` | `helpdesk settings`, `configure`, `directory` | `helpdesk_settings`, `helpdesk_configure`, `helpdesk_directory` |
| Ticket list/read/status/assignment/archive/revocation | Yes | `/tickets`, `/tickets/{id}` | `helpdesk list`, `get`, `update` | `helpdesk_list`, `helpdesk_get`, `helpdesk_update` |
| Contact and article references | Yes | `/tickets/{id}/links` | `helpdesk link` | `helpdesk_link` |
| Staff messages and internal notes | Yes | `/tickets/{id}/messages` | `helpdesk messages`, `reply` | `helpdesk_messages`, `helpdesk_reply` |
| Customer creation/read/reply | Yes | `/api/v1/public/helpdesk/intake/{key}`, `/tickets/{id}`, `/tickets/{id}/messages` | Deliberately omitted | Deliberately omitted |
| Staff business export | Yes | Shared exports | Shared export commands | Shared export tools |

Staff HTTP paths have the `/api/v1` prefix. Mutating staff CLI commands read their typed JSON from stdin and require `--idempotency-key`. Public browser/API operations are the supported customer interfaces; the first-party CLI/MCP credential store remains a staff-session mechanism and does not store ticket codes.

`helpdesk_tickets` and parent-scoped `helpdesk_messages` use the ordinary staff read scopes plus `workspace.data.export`. The staff message export includes internal notes. Codes, their hashes and retry fingerprints are never exported. PostgreSQL backup includes every Helpdesk table, with no extra service, file store, worker or recovery procedure. Codes survive a restore, so keep a restored installation isolated until cutover.

## Verification

`scripts/helpdesk_smoke.py` exercises real PostgreSQL and HTTP with signed-in staff under different grants: standalone use, two-ticket isolation, private-note projections and cursors, concurrent create/reply retries, staff permission removal and replay denial, version conflicts, archive/revocation/module behavior, contact merges, cross-workspace references, restricted article previews, contact timeline, export, required-audit rollback, immutable message privileges and CLI/MCP parity.

`scripts/helpdesk_browser_smoke.py` drives the rendered forms through the gateway: application selection, intake, validation preserving drafts/codes, scoped cookies, a second browser entering its code, public/private messages, CSRF refusal, escaped markup, status/reopening and revocation. Both are part of `scripts/acceptance.py`.

`scripts/helpdesk_chrome_smoke.mjs` is an optional actual-Chrome check using Node's built-in CDP/WebSocket support. It performs real form submissions, verifies stored markup does not inject DOM or execute, checks private-note separation and narrow-screen overflow, and writes temporary review screenshots. It requires a local Chrome executable; `CHROME_PATH` overrides the macOS default.

`scripts/recovery_smoke.py` additionally seeds public/private Helpdesk history and checks both the staff projection and saved customer code after an isolated restore. The upgraded and freshly initialized schema-34 Helpdesk API and browser journeys passed, as did the actual-Chrome run. Rust unit tests (39), Python unit tests (28), Rust formatting/Clippy, frontend formatting/type checking and OpenAPI drift checking passed. All application acceptance suites passed, followed by the authentication-mail, invitation-mail and diagnostics suites. The last three were rerun after correcting a frontend-only fixture restart that had omitted its derived mail-enabled setting; no application change was needed. See [build coverage](coverage.md).

The isolated recovery rehearsal passed all 136 table counts, two Files objects, and one quarantined queued job, then verified login, CRM/Tasks/Files writes, CMS, Projects, Knowledge Base and Helpdesk. Its source contained eight tickets and 21 messages across test workspaces. The 608,671,000-byte encrypted portable archive took 23.0 seconds to create, with 11.4 seconds of writer downtime; isolated data/image restore reported 19.0 seconds, excluding subsequent browser and business checks. Public replies and private notes retained their separation, a saved customer code still worked, and a revoked code stayed revoked.

On the Apple M6 local Colima fixture, 30 sequential customer ticket reads through the gateway measured p50 1.1 ms / p95 1.4 ms after upgrade and 1.2 ms / 1.4 ms on the fresh installation. These are small warm samples, not load tests; no tunnel latency is included. Helpdesk adds no process or background worker.

A post-regression `docker stats` snapshot of the ten-service default fixture totaled 1,579.3 MiB: ClickHouse 773.4 MiB, API 417.6 MiB, PostgreSQL 286.5 MiB, and the remaining services 101.8 MiB. That accumulated multi-application fixture held 4,430 contacts, 5,943 shared tasks, 15 tickets and 44 Helpdesk messages. This is whole-stack observed memory at one point, not incremental Helpdesk memory or a clean-install minimum.

Environment notes: this host's Colima port forwarding is IPv4-only, so the fresh fixture used a temporary IPv6 loopback relay, matching the existing acceptance fixture. Recovery used a private workspace-local `TMPDIR` because Colima cannot bind-mount this macOS user's default temporary directory. Neither workaround is packaged as a new installation guarantee. The additional test installations and temporary relay were stopped after verification; encrypted recovery evidence was retained.

Not supported: email intake or delivery, email recovery, attachments, message editing/deletion, permanent ticket deletion, customer account directories, authenticated customer memberships, public knowledge articles, SLAs, team queues, notifications, automation, custom statuses, ticket merging or a general customer portal. Email remains a separately tested integration with an existing provider.

# CRM planning record

This is the working product/build plan for CRM as of October 4, 2026, not an ADR or a completed implementation specification. It owns evolving CRM scope, examples, proposed behavior, open questions, and planning history. The [architecture](architecture.md) owns shared technical requirements; the [build plan](build-plan.md) owns delivery gates. Items marked proposed still need agreement or evidence.

## Accepted direction

- Keep opportunities (potential business) and deals (concrete live or historical transactions/engagements) as distinct records with independent lifecycles. A deal can originate from an opportunity or be created directly.
- Use import contacts → qualify interest → track an opportunity → follow up → win/lose as a reference acceptance journey. It is not a mandatory sequence or a service-industry restriction. A business can use Contacts without sales records, create an opportunity directly, and record communications without creating a lead/deal.
- Support user-defined CRM statuses using stable IDs, editable labels/order, and sales-outcome mappings. Renaming is not a business transition. General custom fields remain a separate scope choice.
- Support tags on contacts and other relevant records, beginning with CRM records; tags are independent labels, not statuses or permission grants.
- Defer Gmail and CRM communication implementation beyond the initial foundation/CRM build. Preserve the omnichannel direction and email connector notes for later planning; neither mailbox sync nor business-email sending is an initial release prerequisite. Authentication emails remain part of the foundation.
- Design for agents and just-in-time data-loading scripts. Defer a dedicated bulk-import product and CSV wizard; scripts use supported CLI/API operations with normal validation, authorization, and retry guarantees.
- People who share an email address or phone number remain separate contacts. Ambiguous matches require review.
- Support an explicit path from an existing contact to a login account and scoped access, without conflating the contact and authenticated user. Delivery timing and portal access policies remain open.
- Authorization belongs in Rust business operations; business writes and required audit/job scheduling commit atomically in PostgreSQL. Follow-up work uses PGMQ. CLI, API, and MCP use those same operations.
- Local ClickHouse is the selected operational log store. It does not replace the transactional business database or its authoritative audit history.
- RBAC presets and record visibility remain to be designed. Administrator/member/read-only are examples, not accepted final roles.

## Opportunities, deals, and optional lead qualification

Accepted distinction: an opportunity is potential business; a deal is a concrete transaction or engagement that can be live, completed, cancelled, or historical. Deals may support transaction coordination and do not require a signed contract to exist. The earlier use of opportunity/deal as synonyms is superseded.

Use separate opportunity and deal records/tables with independent IDs, statuses, history, tags, participants, and links to conversations. Opportunity outcome describes the sales pursuit; deal status describes the actual transaction or engagement. Winning an opportunity is not the same event as completing a deal.

Proposed relationship: an opportunity can produce zero or more deals, and a deal can be created without an opportunity. Start with an optional originating-opportunity reference on a deal if one origin is sufficient; validate whether multiple origins are needed before schema finalization. Creating a deal from an opportunity should preserve the original opportunity and copy/link only deliberate fields; it must be atomic and retry-safe. Direct creation supports businesses recording existing transactions or coordinating work without an earlier sales funnel.

Example: an opportunity to renovate an office can produce a renovation deal and a maintenance deal. Each deal has its own participants, dates, tasks, milestones, and history. Exact coordination features are CRM scope choices; invoicing, payment execution, contracts/signatures, and project delivery remain owned by their respective modules when implemented.

A lead remains potential interest being qualified; a dedicated lead inbox/conversion object is optional and unresolved. Do not require every contact, inquiry, or conversation to produce an opportunity. Custom opportunity stages may cover qualification initially. This is a product/schema choice recorded here, not a new ADR.

## Proposed contact model and matching rules

Use stable contact IDs scoped to a workspace. Store multiple email addresses and phone numbers per contact, with originals, normalized matching values, labels, provenance, and verification state where applicable. E.164 applies to telephone numbers; channel-specific provider identities are separate and scoped to their provider/account. A username or message sender label is not proof that two people are the same.

Allow a person to relate to multiple organizations, with relationship role and primary association as optional metadata. Organization deduplication is separate from person matching; a company name or email domain alone is not a reliable unique identifier.

Accepted handling of shared identifiers: retain separate people when they share a mailbox or phone number and review ambiguous intake. Do not impose a unique-per-person email/phone constraint that prevents this. Proposed implementation: a normalized identifier registry with unique keys within a workspace and associations to contacts that support reviewed sharing.

| Matching result | Proposed behavior |
| --- | --- |
| Authorized explicit contact ID | Use that contact; conflicting new details still need validation. |
| All matching identifiers resolve to one contact | Reuse it; unverified input must not silently attach trusted identifiers or overwrite fields. |
| Email matches A and phone matches B, or an identifier matches several contacts | Preserve the source input and request review; do not merge automatically. |
| Valid identifiers match nothing | Create a contact through a concurrency-safe resolution operation. |
| No usable identifier | Preserve the inquiry/submission; offer manual resolution or an explicitly provisional contact. Never merge by name alone. |

Trim email whitespace and lowercase the domain; decide local-part case normalization explicitly. Do not globally strip plus tags or dots. Parse phone numbers using known country context, keep extensions separately, and exclude invalid numbers from automatic matching.

Matching/creation must coordinate concurrent requests on normalized identifier keys, including multi-key conflicts, with a defined locking order and database constraints where appropriate. An idempotency key prevents repeat execution of one request; it does not resolve two different requests mentioning the same person.

Proposed merge flow: a permitted user previews records/fields and relationships, chooses the surviving contact, resolves conflicting fields, and confirms. Preserve original source messages/submissions and a merge audit trail; maintain references or aliases so old links resolve. Define unmerge feasibility before promising undo, especially once later changes or external actions have occurred. Revocation or alteration of login identities is not a contact merge operation.

## Contacts and authenticated users

Contacts represent people the business knows; application users represent authenticated actors. Contacts do not automatically get Supabase accounts, and users do not require contact records. Supabase identity maps by issuer and subject to the stable application user ID; email is not the foreign key between these concepts.

Support creating an explicit, permission-checked user/contact link in application-owned data scoped to the workspace and purpose when granting a contact login access. Never infer it solely from equal email/phone values. Two contacts sharing a mailbox do not create two accounts with the same login email. Control of that mailbox identifies the authenticated account, not which of the two people is acting; independent access and attribution require distinguishable login identities. Permission to represent or view another contact is a separate grant, not an identity merge.

For example, Alex and Sam may both list `office@example.com` for correspondence. Alex can sign in using a distinct personal login and be explicitly linked to Alex's contact. Sam remains a separate contact. A magic link sent to the shared mailbox must not automatically expose both contacts' records. Changing a contact email, merging contacts, or deleting a contact must not implicitly change credentials, combine users, or grant/revoke workspace membership. Association cardinality, proof requirements, and delivery timing remain open; the ability to bring contacts into login flows is now required product direction.

Proposed contact-to-login flow:

1. An authorized operator selects the contact and intended access purpose, such as future customer-portal access, plus a suitable login address. It need not equal every correspondence address on the contact.
2. Create a pending, expiring invitation bound server-side to that contact ID, workspace, intended recipient, purpose, and proposed access. Do not embed arbitrary caller-controlled grants in a signup request.
3. Supabase authenticates an existing user or handles authorized account creation. The backend checks the pending invitation, recipient binding, expiration/revocation, and any identity ambiguity; verified mailbox control alone is not sufficient to distinguish people sharing it.
4. Atomically consume the invitation, link the stable application user to the specified contact, and grant only the approved access. Replays must not duplicate the association or elevate access. Customer access must not default to employee/member access.
5. Audit the link and expose a controlled unlink/revoke path. Existing identities are reused; contact creation, email matching, or a contact merge never creates or combines login accounts automatically.

## Custom-status behavior

Status definitions belong to a workspace and the applicable record type/pipeline. Use stable IDs, editable labels, ordering, and an archive state. Opportunity stages also map to semantic outcomes such as open/won/lost so reports and automations do not interpret label text. Deals have their own lifecycle and custom statuses; proposed categories include active/completed/cancelled, with exact semantics still to be designed. A won opportunity can therefore have an active deal. Neither lifecycle is a universal status enum for every module.

Renaming a status preserves references and does not trigger a business transition. Referenced statuses cannot simply disappear: archive them or migrate records with a preview. Real transitions write history and may enqueue configured follow-up. Moving between pipelines validates or explicitly maps the destination stage. Exact transitions, permissions, multiple-pipeline scope, and treatment of historical labels remain open.

## Omnichannel communication plan

Status: deferred beyond the initial foundation/CRM build. The following is retained design context for when communication work is scheduled, not current implementation scope. Do not build mailbox connectors, inbox UI, or a speculative communications schema as prerequisites for CRM. Authentication email delivery is separate and remains in scope.

When communication work resumes, start with email while retaining an omnichannel model. Gmail and generic SMTP remain candidate connection paths from the discussion. Reconfirm scope and provider requirements then. Previously proposed connector choices:

| Connection | Sending | Incoming/sync | Credentials |
| --- | --- | --- | --- |
| Gmail / Google Workspace | Gmail API | Gmail API messages, threads, and incremental history | Google OAuth authorization and securely stored refresh credentials. |
| Generic mailbox | SMTP submission | IMAP mailbox access | Provider-supported authentication over TLS; incoming and outgoing settings may differ. |
| SMTP only | SMTP submission | None unless separately configured | Provider-supported SMTP credentials or explicitly configured trusted relay. Clearly show send-only capability. |

Google authorization here lets the suite access a mailbox. It is distinct from Supabase login and from offering our own OAuth server to external developers; that deferred OAuth-server work is not a prerequisite. Resolve Google OAuth client provisioning, redirect URLs for self-hosted instances, required scopes, and applicable verification requirements before distributing the connector. Reuse maintained protocol/auth implementations rather than implementing OAuth cryptography or a mail server.

Treat mailbox connections as first-class records with owner/workspace access, sending identities, capabilities, sync state, encrypted credential references, and reconnect/disconnect behavior. Distinguish personal from shared/team mailboxes; connecting a personal mailbox must not make every message workspace-visible. Determine folder/label/date selection, initial backfill, sharing rules, attachments, and whether read/delete/label changes sync back before claiming full two-way synchronization.

Use asynchronous sync and sending through PGMQ. Track durable provider-specific cursors and deduplicate within the connected account. Gmail history expiration and generic mailbox UID validity changes require explicit resync handling. Preserve email headers and provider thread IDs for threading; do not group messages only by subject or shared contact address. SMTP acceptance is not proof of delivery, and SMTP alone provides neither inbound messages nor a universal exactly-once send guarantee. Handle uncertain sends without blindly retrying them.

Login/invitation SMTP configuration does not by itself constitute a CRM email inbox. Its sender, purpose, and access should remain separate from connected business mailboxes even if the same underlying provider is used. The proposed implementation uses Gmail API for the Gmail connector, though Gmail also supports OAuth-authenticated IMAP/SMTP.

References: [HubSpot inbox connections](https://knowledge.hubspot.com/connected-email/connect-your-inbox-to-hubspot?lang=en), [Pipedrive email sync](https://support.pipedrive.com/en/article/email-sync), [Gmail API](https://developers.google.com/workspace/gmail/api/guides), [Gmail protocols](https://developers.google.com/workspace/gmail/imap/imap-smtp), [Gmail synchronization](https://developers.google.com/workspace/gmail/api/guides/sync), and [Gmail scopes](https://developers.google.com/workspace/gmail/api/auth/scopes).

Propose a shared Communications module for channel accounts, conversations, participants, messages, attachments, and delivery state. CRM links conversations to contacts, opportunities, and deals; Helpdesk can later link them to tickets without becoming a second owner of the same message. Connector implementation and the module's exact interface need design before implementation.

Keep provider-specific IDs and metadata, threading, participants, and delivery capabilities. A common inbox/timeline does not mean all channels support the same threading, edit, delivery, or attachment behavior. Linking messages to the same contact must not automatically collapse separate conversations. Distinguish sending from receipt, provider acceptance, and confirmed delivery; outbound messages run through the queue. Authenticate provider callbacks and deduplicate incoming events by provider/account/event identity.

Store authoritative conversations and messages in PostgreSQL, with attachment bytes in file storage. ClickHouse receives redacted diagnostic metadata, not the only copy of communications or default full message bodies. Channel availability and any external delivery costs remain explicit dependencies.

## Tags

Tagging is accepted for contacts, opportunities, deals, and other relevant business records. Accepted starting design: one workspace tag catalog with stable IDs, editable labels and optional colors, reused by modules that support tagging. A record can have many tags; a tag can label many records. Define name normalization and duplicate-name behavior before implementation.

Keep assignment tables inside each owning module, such as contact-tag and deal-tag associations, with foreign keys, workspace checks, and uniqueness per record/tag. This retains referential integrity while sharing the tag vocabulary; avoid an unrestricted `(table_name, row_id)` association that cannot validate its targets. These are proposed table shapes, not a finalized migration.

Create/rename/archive tags through the catalog interface; attach/detach them through the record owner's authorized business operation. Tags do not grant access or override lifecycle state. Filtering and usage counts must respect record permissions and enabled modules. Stable IDs preserve links when labels change; define archive/removal semantics for assigned tags. Offer consistent tagging and filtering through UI, API, CLI, and MCP. Supported record types grow with actual needs rather than making every database table taggable.

## Agent-driven data loading, export, and record lifecycle

Defer a dedicated import wizard, import mapping UI, and resumable bulk-import framework. Agents can inspect source data and write task-specific scripts against supported CLI/API operations. Preserve machine-readable output, bounded pagination, clear validation errors, stable record IDs, request idempotency, and concurrency protections. Scripts never write directly to module tables or bypass permissions, contact matching, or audit history.

A script can normalize/map input, preview its intended changes, keep its own checkpoint/result file, and retry failed operations. A failed run may have partially completed; do not claim whole-file atomicity or automatic rollback. Reuse operation keys when retrying the same intended write and account for server-side idempotency retention. Separate creation from updates; changed data and ambiguity still require explicit resolution. Add server-side batching or job support later if measured workloads justify it.

Use archive as the ordinary reversible removal action; permanent deletion is a separate permission-checked operation with a defined reference and retention policy. Archived contacts must be considered during matching to prevent accidental recreation, without silently reactivating them. Merge, deletion, privacy/retention, and audit visibility require further decisions. Do not promise arbitrary rollback of an import after records have been edited or external actions performed.

Exports should include permitted stable IDs, relationships, custom statuses, and custom fields with a manifest. Decide attachment/message export scope and representation before claiming full portability. Spreadsheet-oriented exports should account for formula injection while retaining a documented faithful machine-readable path.

## Proposed programmatic contract

These are concrete defaults to review, not implemented endpoints or a frozen public API.

| Concern | Proposed contract |
| --- | --- |
| Addressing | Versioned HTTP paths such as `/api/v1/workspaces/{workspace_id}/…`; stable opaque record IDs. Verify membership/resource scope, not merely the path value. |
| Reads | Bounded cursor pagination with deterministic ordering; explicit filters/sorts; one documented timestamp/date and monetary representation. |
| Errors | Stable machine codes, safe message, field errors when applicable, and request ID; consistent meaning across HTTP, CLI, and MCP. |
| Retried mutations | `Idempotency-Key` on eligible HTTP mutations, with equivalent CLI/MCP inputs. Store scope, payload fingerprint, result, and business writes atomically. Reject key reuse with changed input; recheck access before returning a saved result. Define retention and behavior for in-flight/expired keys. |
| Concurrent edits | Expected record version; HTTP can carry it through `If-Match`, CLI/MCP through `expected_version`. Reject stale changes rather than silently overwrite them. |
| Data-loading scripts | Compose ordinary authorized operations, with per-operation results, safe retries, and client checkpoints. Dedicated bulk-import jobs/UI are deferred. |
| CLI | Explicit installation/workspace, JSON output mode, stable exit codes, stderr diagnostics, and secure credential handling. |
| MCP | Task-oriented tools with structured inputs/results and bounded output; no arbitrary SQL. Same business operations and permission checks. |
| Evolution | Additive changes within a compatible version; document deprecation and breaking changes. Verify generated OpenAPI against implemented behavior. |

## Planning history

- October 4, 2026: accepted a reference journey while requiring general applicability; accepted custom statuses and omnichannel direction; left RBAC details open. Confirmed business-operation authorization, atomic writes/queued follow-up, and local ClickHouse logging. Confirmed that shared email/phone identifiers may belong to separate contacts and ambiguous matches require review; chose to design the communication model now and select channels later. Detailed normalization/merge behavior, imports, and programmatic contracts remain under discussion. Routine product decisions belong here and in the build plan; ADRs are reserved for consequential architecture trade-offs.

- October 4, 2026, refinement: accepted the custom-status starting model and tagging, chose email as the first communication channel, and deferred dedicated bulk-import tooling in favor of agent-authored scripts. Clarified that contacts are independent of login accounts and that opportunity/deal table separation remains under discussion.

- October 4, 2026, clarification: opportunities remain potential business; deals are separate live/historical transaction records and may support coordination. Added an explicit contact-to-login requirement, selected Gmail plus generic SMTP connectivity as email direction, and accepted the shared tag-catalog starting design. Opportunity/deal cardinality, exact email sync scope, and customer access policy remain open.

- October 4, 2026, scope reduction: deferred Gmail and communication implementation until that work is scheduled. Retained connector research as future context, removed it as an initial CRM release requirement, and kept authentication SMTP/invitations/magic links in the foundation.

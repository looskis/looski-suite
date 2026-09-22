# CRM planning record

This is the working product/build plan for CRM updated October 12, 2026, not an ADR or a completed implementation specification. It owns evolving CRM scope, examples, proposed behavior, open questions, and planning history. The [architecture](architecture.md) owns shared technical requirements; the [build plan](build-plan.md) owns delivery gates. Items marked proposed still need agreement or evidence.

The [schema design draft](schema-design.md) translates this plan into proposed tables, constraints, and migration groups. It does not silently settle the open product choices below.

## Accepted direction

- Keep opportunities (potential business) and deals (concrete live or historical transactions/engagements) as distinct records with independent lifecycles. A deal has zero or one originating opportunity; an opportunity can produce multiple deals. Direct deal creation is supported.
- Use import contacts → qualify interest → track an opportunity → follow up → win/lose as a reference acceptance journey. It is not a mandatory sequence or a service-industry restriction. A business can use Contacts without sales records, create an opportunity directly, and record communications without creating a lead/deal.
- Support user-defined CRM statuses using stable IDs, editable labels/order, and sales-outcome mappings. Renaming is not a business transition. General custom fields remain a separate scope choice.
- Make deal categories, activity behavior, and transaction coordination workspace-configurable. Ship a standard starter template that can be edited, replaced, or removed. No separate lead table: qualification uses opportunities and their stages.
- Use a shared Tasks capability with dedicated task records and typed links from CRM/Contacts. Projects later adds grouping and boards. Coordination templates use supported task, status, checklist/milestone, and trigger/action building blocks; removing a template preserves its instances.
- Support tags on contacts and other relevant records, beginning with CRM records; tags are independent labels, not statuses or permission grants.
- Defer Gmail and CRM communication implementation beyond the initial foundation/CRM build. Preserve the omnichannel direction and email connector notes for later planning; neither mailbox sync nor business-email sending is an initial release prerequisite. Authentication emails remain part of the foundation.
- Design for agents and just-in-time data-loading scripts. Defer a dedicated bulk-import product and CSV wizard; scripts use supported CLI/API operations with normal validation, authorization, and retry guarantees.
- People who share an email address or phone number remain separate contacts. Ambiguous matches require review.
- Support an explicit path from an existing contact to a login account and scoped access, without conflating the contact and authenticated user. Future customer access uses separate workspace-scoped external memberships, external roles, and explicit record/action grants. Recipient proof, detailed portal policies, and delivery timing remain design work.
- Authorization belongs in Rust business operations; business writes and required audit/job scheduling commit atomically in PostgreSQL. Follow-up work uses PGMQ. CLI, API, and MCP use those same operations.
- Local ClickHouse is the selected operational log store. It does not replace the transactional business database or its authoritative audit history.
- Enforce RBAC with explicit record scope on permission grants. Reuse the existing workspace roles, permission catalog, and membership-role assignments; do not infer access from role names. Preset contents and any team-specific policy remain design work.
- Preserve email local-part case. Merge confirmed duplicates by selecting a survivor, resolving conflicts, and preserving relationships/history. Archive by default; permanent deletion requires its own permission. No automatic record deletion by default.

## Opportunities, deals, and optional lead qualification

Accepted distinction: an opportunity is potential business; a deal is a concrete transaction or engagement that can be live, completed, cancelled, or historical. Deals may support transaction coordination and do not require a signed contract to exist. The earlier use of opportunity/deal as synonyms is superseded.

Use separate opportunity and deal records/tables with independent IDs, statuses, history, tags, participants, and links to conversations. Opportunity outcome describes the sales pursuit; deal status describes the actual transaction or engagement. Winning an opportunity is not the same event as completing a deal.

Accepted relationship: an opportunity can produce zero or more deals; each deal has zero or one originating opportunity. Model this as nullable `crm.deals.originating_opportunity_id`, with a workspace-qualified foreign key and a non-unique index on `(workspace_id, originating_opportunity_id)`. No origin join table is needed for this relationship. Creating a deal from an opportunity should preserve the original opportunity and copy/link only deliberate fields; it must be atomic and retry-safe. Direct creation supports businesses recording existing transactions or coordinating work without an earlier sales funnel.

Example: an opportunity to renovate an office can produce a renovation deal and a maintenance deal. Each deal has its own participants, dates, tasks, milestones, and history. Exact coordination features are CRM scope choices; invoicing, payment execution, contracts/signatures, and project delivery remain owned by their respective modules when implemented.

There is no separate lead table or lead-conversion object. Lead describes an opportunity being qualified; configurable opportunity stages handle qualification. Do not require every contact, inquiry, or conversation to produce an opportunity. This is a product/schema choice recorded here, not a new ADR.

## Staff roles and record visibility

These answer different questions. A role grants actions such as reading contacts, editing deals, exporting data, or managing members. Visibility determines the records on which those actions are allowed: all eligible records in the workspace, records assigned to the user, or records available to their team. Membership in workspace A never grants access to workspace B.

For example, a salesperson may edit deals but only within their team; a read-only member may see all eligible workspace records while editing none. Ownership records responsibility; it does not automatically define visibility. Search, matching, exports, timelines, counts, and linked records must use the same visibility policy as direct reads.

Accepted design: RBAC determines actions and each permission grant carries an explicit record scope. Reuse `suite.roles`, `suite.permissions`, `suite.role_permissions`, and `suite.membership_roles`. Extend the role-permission relation when the first scoped CRM operation ships; do not create a second generic roles/access table with overlapping authority. A role can allow `crm.deals.read` across a workspace and `crm.deals.update` only for assigned deals. A single scope on the entire role cannot express this correctly.

Scopes are a finite, code-supported vocabulary: workspace, assigned, team, or explicitly granted records as the corresponding policies are implemented. Unimplemented scopes deny access; do not expose configuration before its queries and denial tests exist. Teams require actual team memberships and record-team relationships; a role label is not a team. For the same action, allowed records from multiple applicable roles combine by union; active actor/membership/workspace, enabled module, resource rules, and any future delegated grant still restrict the result. Never combine a broad read scope with an unrelated edit permission to create broad editing rights. No implicit allow or wildcard future permissions.

Keep authorization in module business operations, with scoped queries applied before pagination/counts. Role administration must restrict who can assign/manage each grant, prevent self-escalation, audit changes, clear assignments on removed memberships, and protect the final administrator. Preset contents and team semantics remain implementation design work. Contacts and base CRM records now enforce workspace/assigned scopes per action; team policies remain unimplemented. See the [implementation checkpoint](implementation/contacts-slice.md).

Permanent deletion is a separate permission such as `contacts.delete_permanently`, independent of `contacts.archive`, `contacts.update`, and export. A future OAuth scope can further restrict this permission but cannot create it. Destructive operations still enforce resource scope and reference/retention rules; no implicit cascading destruction of linked records.

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

Accepted email matching policy: trim surrounding whitespace, lowercase the domain, and preserve local-part case. Do not globally strip plus tags or dots. Parse phone numbers using known country context, keep extensions separately, and exclude invalid numbers from automatic matching.

Matching/creation must coordinate concurrent requests on normalized identifier keys, including multi-key conflicts, with a defined locking order and database constraints where appropriate. An idempotency key prevents repeat execution of one request; it does not resolve two different requests mentioning the same person.

Accepted merge flow: a permitted user previews records/fields and relationships, chooses the surviving contact, resolves conflicting fields, and confirms. Preserve original source messages/submissions and a merge audit trail; maintain references or aliases so old links resolve. Define unmerge feasibility before promising undo, especially once later changes or external actions have occurred. Revocation or alteration of login identities is not a contact merge operation.

### Normalization, merge, archive, and retention explained

The email local part is the text before `@`. Accepted: `Alex@example.com` and `alex@example.com` remain distinct exact matching keys. The policy is: preserve the original, normalize the domain, retain local-part case/dots/plus tags, and suggest case-only matches for review rather than silently conflating identities.

Merge means consolidating records confirmed to represent the same person: choose a surviving contact, resolve field conflicts, preserve business links/history, and retain an alias for old IDs. Sharing an address alone is never grounds for merging. Archive means hide from ordinary active lists while retaining history and a restore path; archived identifiers still participate in controlled matching. Retention means deciding when archived records, original inputs, audit evidence, and backups may be purged. Accepted default: archive rather than hard-delete, require a separate permanent-deletion permission, and perform no automatic purge of CRM records; backup expiry is a separate policy. Detailed erase/unmerge behavior remains open.

## Contacts and authenticated users

Contacts represent people the business knows; application users represent authenticated actors. Contacts do not automatically get Supabase accounts, and users do not require contact records. Supabase identity maps by issuer and subject to the stable application user ID; email is not the foreign key between these concepts.

Support creating an explicit, permission-checked user/contact link in application-owned data scoped to the workspace and purpose when granting a contact login access. Never infer it solely from equal email/phone values. Two contacts sharing a mailbox do not create two accounts with the same login email. Control of that mailbox identifies the authenticated account, not which of the two people is acting; independent access and attribution require distinguishable login identities. Permission to represent or view another contact is a separate grant, not an identity merge.

For example, Alex and Sam may both list `office@example.com` for correspondence. Alex can sign in using a distinct personal login and be explicitly linked to Alex's contact. Sam remains a separate contact. A magic link sent to the shared mailbox must not automatically expose both contacts' records. Changing a contact email, merging contacts, or deleting a contact must not implicitly change credentials, combine users, or grant/revoke workspace membership. Association cardinality, proof requirements, and delivery timing remain open; the ability to bring contacts into login flows is now required product direction.

Proposed contact-to-login flow:

1. An authorized operator selects the contact and intended access purpose, such as future customer-portal access, plus a suitable login address. It need not equal every correspondence address on the contact.
2. Create a pending, expiring invitation bound server-side to that contact ID, workspace, intended recipient, purpose, and proposed access. Do not embed arbitrary caller-controlled grants in a signup request.
3. Supabase authenticates an existing user or handles authorized account creation. The backend checks the pending invitation, recipient binding, expiration/revocation, and any identity ambiguity; verified mailbox control alone is not sufficient to distinguish people sharing it.
4. Atomically consume the invitation, link the stable application user to the specified contact, and grant only the approved access. Replays must not duplicate the association or elevate access. Customer access uses a separate workspace-scoped external membership, constrained by both its external role and explicit record/action grants. It never defaults to staff permissions or extends staff membership implicitly.
5. Audit the link and expose a controlled unlink/revoke path. Existing identities are reused; contact creation, email matching, or a contact merge never creates or combines login accounts automatically.

### Accepted external access relationship

Keep a contact as a business record. Add access for an authenticated user through a separate workspace-scoped external membership, not by treating the contact row as a credential or silently creating staff membership. Accepted future relations are `suite.external_memberships` (workspace, user, active/revoked state), external role assignments, and module-owned typed grants such as `crm.deal_external_grants` (workspace, deal, external membership, allowed action). The explicit contact-to-user link remains a separate representation/identity relationship.

Reuse the role/permission vocabulary and authorization machinery, with an enforced staff/external role audience and code-defined external-eligible permissions. An external grant must satisfy both the assigned external role and the per-record allowed action; grant recipients cannot acquire staff roles or workspace-wide CRM scope. Use dedicated shared-data views/actions where necessary: access to one deal must not expose internal notes, private attachments, unrelated participants, or linked records. Revocation blocks the next read even with a valid Supabase session. The same person may be staff in one workspace and a customer in another, or hold distinct staff/external relationships without those roles being implicitly combined.

This is the accepted design for the future customer-access feature, not a shipped invitation or portal. Only create its tables with the first authorized external-access operation. Do not add a generic `entity_type/entity_id` grant table without referential integrity.

## Custom-status behavior

Status definitions belong to a workspace and the applicable record type/pipeline. Use stable IDs, editable labels, ordering, and an archive state. Opportunity stages also map to semantic outcomes such as open/won/lost so reports and automations do not interpret label text. Deals have their own lifecycle, workspace-defined categories, and custom statuses. Active/completed/cancelled are possible starter-template values, not required database enums or permanent categories. A won opportunity can therefore have an active deal. Neither lifecycle is a universal status enum for every module.

Renaming a status preserves references and does not trigger a business transition. Referenced statuses cannot simply disappear: archive them or migrate records with a preview. Real transitions write history and may enqueue configured follow-up. Moving between pipelines validates or explicitly maps the destination stage. Exact transitions, permissions, multiple-pipeline scope, and treatment of historical labels remain open.

## Workspace configuration and removable starter templates

Accepted: the business can specify its deal categories, activity behavior, and transaction-coordination setup. Ship a standard starter template with useful defaults, while supporting customization, replacement, and removal. A suggested template might contain active/completed/cancelled deal categories, note/call/meeting activity types, dedicated task templates, and a simple checklist or milestone sequence; its exact contents and configuration parameters still need design within the accepted building blocks below.

Configurations use stable workspace-owned definitions; categories and activity types are records rather than fixed enums. Behavior must be explicit configuration over supported capabilities, not inference from labels. For example, renaming an activity to “Invoice” does not implement invoicing, and a workflow definition cannot bypass access checks. This requirement does not implicitly pull the later general-purpose automation builder into the first CRM slice.

Accepted template lifecycle: installing a version copies definitions into the workspace; subsequent edits belong to that workspace. Removing the starter template never deletes deals or activity history, and updates must not silently reinstall removed defaults. Unused definitions can be removed; referenced definitions are retired or records are explicitly remapped. Existing records remain readable without the template, including historical snapshots. Material behavior changes need revision and migration rules so an edited template does not silently change in-progress coordination.

The earlier fixed activity-kind and fixed deal-category proposals are superseded. Exact behavior parameters, supported configuration UI/API, reporting/completion metadata, template revisions, and in-progress migration semantics remain implementation design work.

### Tasks and configuration building blocks

A task is an actionable record with its own assignment, due date, status, and completion lifecycle; it should have a dedicated table rather than live as a variant of a generic CRM activity. Accepted ownership is a small shared Tasks capability (`tasks.tasks`) with typed links from CRM/Contacts and, later, Projects. This revises the earlier task-in-`crm.activities` sketch. CRM must not require the optional Projects application merely to record a follow-up task; Projects later adds project grouping, boards, and delivery views over the same task records. This ownership split is accepted; task tables and operations are not implemented yet.

A CRM activity records something that happened (a note or logged interaction); the timeline composes activities, task events, and status changes through their owning interfaces. It is not a second authoritative copy of task state. Linking a task does not automatically grant access to its parent or make internal task content customer-visible. Cross-module links retain workspace-qualified FKs and each owning operation checks the applicable visibility policies.

The accepted initial configuration building blocks are: task types/templates (what to do, assignment and due-date rules), editable deal categories/statuses and explicit transitions, ordered checklist/milestone definitions, and bounded triggers/actions such as creating a task when a deal reaches a configured stage. Implement only primitives exercised by the first coordination journey. A generic visual workflow builder, arbitrary executable configuration, and dependencies across every kind of record are not implied.

Templates hold reusable definitions. Applying a template creates ordinary task/checklist records and pins the relevant definition revision. Removing a template never deletes those instances; changing its definition does not silently rewrite existing work. Exact task schema, completion conditions, assignment resolution, and retry keys for generated tasks must be designed with the first operation.

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

Use archive as the ordinary reversible removal action; permanent deletion is a separate permission-checked operation with a defined reference and retention policy. Archived contacts must be considered during matching to prevent accidental recreation, without silently reactivating them. Survivor-based merge, archive-by-default, separately authorized permanent deletion, and no automatic record deletion are accepted. Exact reference/erasure handling, unmerge feasibility, backup expiry, and audit visibility require further design. Do not promise arbitrary rollback of an import after records have been edited or external actions performed.

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

- October 12, 2026: accepted zero-or-one opportunity origin per deal and zero-or-many deals per opportunity; no separate lead table. Accepted workspace-defined deal categories, activity behavior, and transaction coordination with an editable/removable standard starter template. Future customer access is always workspace-scoped through a membership or equivalent restricted grant. Staff visibility, normalization, detailed merge/archive/retention rules, and the configuration/access schema remain under refinement.

- October 12, 2026, refinement: confirmed RBAC enforcement and selected scoped permission grants over the existing role model. Accepted email local-part case preservation, survivor-based merge with conflict resolution and retained relationships/history, archive by default, a separate permanent-deletion permission, and no automatic record deletion. Template removal preserves instances. Recommended a dedicated shared Tasks capability and a separate external membership/resource-grant relationship for future customer access; these tables are not implemented.

- October 12, 2026, approval: confirmed scoped RBAC, dedicated shared Tasks ownership and configuration building blocks, and separate external memberships with role-constrained record/action grants. Promoted these designs from recommendations to accepted direction. Precise schema fields, role presets, supported policy details, invitation proof, and template parameters remain implementation design; approval does not imply shipped functionality.

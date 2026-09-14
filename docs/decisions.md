# Decisions and open choices

This register records accepted direction and unresolved choices as of October 4, 2026. Accepted means agreed design, not implemented or verified behavior. The [architecture](architecture.md) owns technical requirements, the [roadmap](roadmap.md) owns application ordering and scope, and the [build plan](build-plan.md) owns milestones and acceptance evidence.

## Accepted decisions

| Decision | Rationale or consequence | Record |
| --- | --- | --- |
| Open-source business applications under AGPL-3.0, prioritizing Apple Silicon self-hosting | Commercial use, resale, and hosting are permitted with copyleft compliance; govern branding through a separate trademark policy. | [License policy ADR](adr/0006-agpl-and-trademark-policy.md) |
| Supabase CLI-managed PostgreSQL migrations | Store all suite SQL migrations in `supabase/migrations/`, apply to an explicit target, and keep one history. SQLx handles queries; upstream Supabase owns `auth` migrations. | [Setup contract](setup-contract.md#migration-ownership) |
| Default local gateway port `58888` | Bind to `127.0.0.1:58888` by default; allow an explicit override and report conflicts. Public HTTPS remains independent of this host port. | [Setup contract](setup-contract.md#canonical-ports-and-endpoints) |
| `looski-suite` installation namespace | Namespace Compose containers/networks/volumes rather than renaming upstream images; use distinct identities and bindings for simultaneous installations. | [Setup contract](setup-contract.md#installation-identity-and-docker-names) |
| Rust with Axum, SQLx, and PostgreSQL | Rust owns business behavior; SQLx transactions coordinate records and queued work. | [Architecture](architecture.md) |
| OpenAPI through `utoipa` and `utoipa-axum` | Generate the HTTP contract from Rust types and endpoint metadata; select TypeScript client tooling and verify drift. | [Architecture](architecture.md) |
| Architecture prepared for future external OAuth API access | Build suite-owned permissions and transport-independent authorization now; defer OAuth server, registration, consent, and grants beyond phases 0 and 1. | [Architecture](architecture.md#external-developers-and-delegated-api-access) |
| CLI, HTTP API, and MCP access to suite capabilities | All interfaces use the same authorized Rust business operations; transport adapters do not bypass module ownership or permissions. | [Architecture](architecture.md#cli-http-api-and-mcp-interfaces) |
| Module-owned DAO pattern over SQLx | Focused repositories/query modules own persistence; application operations own business rules and transaction coordination. No redundant DAO/repository layers or generic CRUD framework. | [Architecture](architecture.md#data-access-responsibilities) |
| Local ClickHouse for operational logs | Collect asynchronously; retain authoritative business audit records in PostgreSQL with the business mutation. | [Architecture](architecture.md#local-observability-and-audit) |
| Generalizable CRM journey, custom statuses/tags, and agent-driven data loading | Reference journey is not mandatory; support custom statuses and tags. Gmail and CRM communications are deferred, as is dedicated bulk-import tooling in favor of agent scripts. Shared identifiers may belong to separate contacts, with ambiguous matches reviewed. | [CRM plan](crm-plan.md) |
| PGMQ for initial background jobs | Use the business database and atomic enqueue; RabbitMQ/BullMQ are future candidates only if needed. | [Workflow ADR](adr/0004-durable-cross-module-workflows.md) |
| Email/password and magic-link login; Cloudflare Tunnel for public access | Configure SMTP, public service URLs, and allowlisted callbacks; application invitations control membership. | [Authentication ADR](adr/0003-authentication-and-authorization.md) |
| Workspace-scoped contact deduplication by email or E.164 phone | Reuse an unambiguous match; separate contacts can share identifiers, with ambiguous matches reviewed. | [CRM plan](crm-plan.md#proposed-contact-model-and-matching-rules) |
| Svelte with SvelteKit, TypeScript, and SSR | Frontend framework selection is complete; Rust retains authoritative business behavior. Integration and supporting libraries still need validation. | [Frontend ADR](adr/0005-sveltekit-frontend.md) |
| Required Supabase Auth dependency in local and hosted modes | Reuse one identity implementation; stable application user IDs remain separate from provider identities. Auth is part of the foundation, not a selectable business application. | [Authentication ADR](adr/0003-authentication-and-authorization.md) |
| Docker and Compose for local Supabase Auth | Accept runtime overhead and depend on pinned upstream images. Supabase owns `auth` and its migrations; suite tables stay outside it. Native distribution and Apple Container are deferred for phases 0 and 1. | [Authentication ADR](adr/0003-authentication-and-authorization.md) |
| Independent local and hosted Supabase instances | One selected instance per application installation, no synchronization or automatic failover. Existing data moves through an explicit migration and cutover. | [Architecture](architecture.md) |
| Application authorization in Rust, backend-only business data access, no duplicate business RLS | Every access path must enforce current workspace and resource permissions; infrastructure must prevent bypass. | [Authentication ADR](adr/0003-authentication-and-authorization.md) |
| Modular monolith in one initial distribution with workspace application selection | Independent application usefulness without requiring separate deployments or custom builds. | [Composition ADR](adr/0001-modular-monolith-and-module-selection.md) |
| Separate opportunity and deal records | Potential business and live/historical transactions have independent identities and lifecycles; direct deal creation is supported. | [CRM plan](crm-plan.md#opportunities-deals-and-optional-lead-qualification) |
| Explicit contact-to-login onboarding | Link by approved invitation and stable IDs, never by matching email alone; customer access is distinct from staff membership. | [CRM plan](crm-plan.md#contacts-and-authenticated-users) |
| Shared Contacts module separate from CRM and login identities | Scheduling, support, and other applications can refer to one business identity without requiring CRM. | [Contacts ADR](adr/0002-shared-contacts-and-module-ownership.md) |
| Module-owned data and small interfaces for business operations | Integrations do not write another module's tables; ownership remains clear as the suite expands. | [Contacts ADR](adr/0002-shared-contacts-and-module-ownership.md) |
| Built-in workflows with durable follow-up work | Booking success does not depend on successful CRM follow-up; failures remain recoverable. | [Workflow ADR](adr/0004-durable-cross-module-workflows.md) |
| Disabling an application preserves its data | Disabling stops access and workflows; data deletion is a separate operation. | [Composition ADR](adr/0001-modular-monolith-and-module-selection.md) |
| First eight applications are the initial roadmap; later seventeen are an ordered expansion backlog | Establish concrete business workflows before building general-purpose application builders. | [Roadmap](roadmap.md) |
| Five-minute setup and one-day builds are goals requiring evidence | Measure installation and disclose foundation reuse and supported replacement scope. | [Product intent](../intent.md) |

## Open choices and required evidence

| Choice | Current direction | Resolve with |
| --- | --- | --- |
| Future external OAuth integration (deferred beyond phases 0 and 1) | Preserve the authorization boundary now; evaluate Supabase OAuth server when implementing delegated access. | Pinned local/hosted support, client registration, consent/grant binding, API permission limits, refresh, revocation, and issuer-specific discovery. Custom scopes and client credentials are not currently supported upstream. |
| CLI/MCP transport and credential UX | Propose CLI and local stdio MCP as authenticated API clients initially; remote OAuth MCP later. | Login/refresh/secure storage, installation/workspace selection, MCP SDK/version, structured errors/results, and capability coverage. |
| TypeScript client tooling and API validation | Axum, SQLx, `utoipa`, and `utoipa-axum` selected. | Select the TypeScript generator; validate success/error schemas, security metadata, stable operation names, and a CI drift check. |
| PGMQ packaging and execution behavior | PGMQ selected in the same PostgreSQL database as business records. | Local/hosted extension compatibility, SQLx atomic enqueue, idempotency, leases, retry/crash tests, backup and restore. |
| Setup ports, routing, and dependency readiness | Local gateway port 58888 selected; private service DNS ports, explicit health/migration gates, and tunnel-to-gateway routing remain proposed. | Validate the [setup contract](setup-contract.md), gateway implementation, configuration generation, tunnel provisioning, conflicts, and two simultaneous installations. |
| Exact local Supabase service set | Start from official Compose configuration with pinned upstream images and standard Auth schema; validate removal of unused services. | Supported dependencies, migrations, upgrades, health checks, measured Apple Silicon footprint. |
| Local-to-hosted migration procedure | Explicit transfer to a separate instance, preserving application identity and membership; plan for sign-in again. | Source/destination version compatibility, Auth and business data transfer, identity mappings, file handling, configuration, cutover and recovery rehearsal. |
| Bootstrap, invitations, recovery, and mail configuration | Supabase email/password plus magic links, application-owned invitations, and configured SMTP. | End-to-end local/hosted flows and safe administrator setup. |
| Initial roles and record visibility | Action-based permissions are accepted; built-in roles and ownership/team visibility rules remain open. | Explicit CRM access matrix covering members, administrators, restricted users, exports, invitations, and deactivated memberships. |
| Multi-module transaction interface | One database transaction for booking and custom intake. | A probe preserving module ownership while committing both records and the event atomically. |
| Contact normalization, conflict handling, and merge policy | Deduplicate by email or E.164 phone within a workspace; protect trusted fields. | Email local-part handling, shared/recycled identifiers, conflicting keys, concurrency, merge audit/undo, and identifier changes. |
| Opportunity/deal relationships and coordination scope | Separate records/tables accepted: potential business versus live/historical transactions or engagements. | Cardinality, creation/linking rules, independent statuses, participants, and coordination features before storage implementation. |
| Communication integration (deferred) | Retain Gmail, SMTP/IMAP, and omnichannel research for later; no initial CRM implementation gate. | Reconfirm scope and choose connector setup, mailbox sharing, sync semantics, and failure handling when communication work is scheduled. Authentication SMTP remains in scope. |
| Contact-to-login onboarding | Explicit invitation/linking path required; contacts remain independent from Auth identities. | Access purpose, recipient proof, shared-address ambiguity, association cardinality, revocation, and implementation milestone. |
| Tag catalog and assignments | Tags on contacts and relevant records accepted; shared workspace catalog and module-owned joins accepted as the starting design. | Naming normalization, permissions, applicable records, filters, and archive/removal behavior. |
| Workflow contracts and configuration | Built-in recipes, recorded revisions, explicit mappings, recoverable runs. | Stable event payloads, idempotency rules, permission model, and replay behavior. |
| Disabled application operations | Preserve data; prevent new work; visibly pause or block dependent runs. | Race handling, administrator experience, explicit resume procedure. |
| Public deployment and calendar integration | Cloudflare Tunnel selected for public routing; provider adapters as needed. | Stable hostname/port layout, SSR cookies, Auth callbacks, reachability, and later calendar conflict/provider failure tests. |
| UI libraries and rich editing | Reuse maintained implementations where appropriate. | Representative interactions and licensing review. |
| License implementation, trademark policy, and release/update policy | AGPL-3.0 selected; separate trademark policy, pinned releases, and tested recovery. | Add standard license text and notices, finalize trademark terms, verify corresponding-source delivery and dependency compatibility, and define the update contract. |
| Local backup policy | Coordinated encrypted database/Auth/queue/file backups and isolated restore are proposed. | Choose schedule, retention, external destination, loss/downtime targets, and demonstrate the [recovery procedure](local-backup-and-recovery.md). |
| Local ClickHouse operations | Local operational log storage selected; collector pipeline proposed. | Pinned ARM64 deployment, ingestion schema, retention/disk bounds, redaction/access, failure behavior, resource measurements, and backup expectations. |
| Performance targets | Small footprint and responsive application behavior. | Establish hardware, dataset sizes, measured baselines, and explicit targets. |

Runtime third-party plugins, physically stripped application bundles, and a general visual automation editor are deferred. Their deferral does not block built-in module selection or built-in workflows.

## Decisions needed for phases 0 and 1

Already accepted: Axum, SQLx, `utoipa`/`utoipa-axum`, PGMQ, email/password plus magic links, Cloudflare Tunnel, AGPL-3.0 with a separate trademark policy, and email/E.164 contact deduplication. The remaining rows are proposed defaults and decision priorities, not additional accepted choices. Phase 0 corresponds to milestone 0 in the build plan; phase 1 is the first CRM release.

| When | Decision | Proposed starting point |
| --- | --- | --- |
| Phase 0 | Client generator and integration | Select TypeScript client tooling, pin compatible versions, and check contract drift in CI. |
| Phase 0 | Queue integration | Validate PGMQ extension packaging and atomic SQLx scheduling against crash/retry and restore tests. No separate broker is needed initially. |
| Phase 0 | Installation and access contract | Use the [setup contract](setup-contract.md): one tested Docker/Compose environment, `looski-suite` namespace, private service ports, configurable gateway binding, health/migration gates, and consistent Auth/tunnel URLs. |
| Phase 0 | Authentication experience | Implement the accepted email/password and magic-link methods with application-owned invitations and SMTP; validate callbacks and SSR sessions through the tunnel. Defer social login and SSO unless a first customer needs them. |
| Phase 0 | Apply the selected license and finalize trademark policy | Add standard AGPLv3 text and notices, verify dependency compatibility and source delivery, and define brand-use terms without prohibiting compliant commercial use. |
| Before phase 1 schema is finalized | CRM scope and sales lifecycle | Start with contacts, organizations, sales opportunities and separate deals, custom stages, tags, activities, notes, and scriptable data access/export; communication connectors are deferred. Decide whether a separate lead inbox/conversion lifecycle belongs in v1; the broad roadmap does not settle that detail. |
| Before phase 1 schema is finalized | Contact matching | Implement accepted email/E.164 matching, settle exact email normalization and shared-identifier representation, and define conflict/merge behavior. Do not silently merge conflicting contacts. |
| Before phase 1 authorization is implemented | Roles and visibility | Begin with administrator, member, and read-only role presets over action permissions. Decide whether members see all CRM records or only assigned/team records. |
| Before phase 1 release | Files, backup, and updates | Use persistent file storage initially; coordinate database and file backup, verify restore on a clean installation, and define recovery after failed upgrades. Use the [local backup proposal](local-backup-and-recovery.md); exact retention and recovery targets remain open. |

Calendar providers and the full booking transaction wait until scheduling; the general workflow builder remains roadmap item 23. Detailed form mappings belong to phase 2. Local-to-hosted migration must be tested before it is advertised, but does not require automatic synchronization or a native installer. Supporting UI library choices and performance targets can be settled through the phase 0 CRM probe.

## Decisions before detailed CRM planning

The [CRM planning record](crm-plan.md) owns the evolving product choices: the generalizable reference journey, lead/deal model, contact relationships and shared identifiers, custom statuses, communication model, imports, and programmatic contract. These are build/product decisions, not individual ADRs. RBAC and record visibility remain open. Gmail and CRM communication work are deferred until that capability is scheduled; authentication email remains required. Dedicated bulk-import tooling is deferred in favor of agent scripts.

The accepted engineering direction is authorization inside Rust business operations, atomic business/audit/job scheduling in PostgreSQL, and queued follow-up. Local ClickHouse is selected for diagnostics; database isolation, request idempotency, and edit-conflict handling are distinct responsibilities. See the [architecture](architecture.md#concurrency-and-retry-semantics).

Backup policy, supported service versions, collector/client/MCP libraries, and measured resource targets belong to implementation and release validation. They do not all need to block CRM product planning.

## Technical references for current recommendations

- [Axum](https://docs.rs/axum/latest/axum/)
- [SQLx](https://docs.rs/sqlx/latest/sqlx/)
- [Utoipa Axum integration](https://docs.rs/utoipa-axum/latest/utoipa_axum/)
- [SvelteKit rendering options](https://svelte.dev/docs/kit/page-options)

## Updating decisions

When an open choice is resolved, record the evidence and date here and update the owning document. Keep routine feature decisions, examples, and planning history in the relevant product/build plan; reserve ADRs for consequential architecture trade-offs. Add an ADR for consequential trade-offs whose rationale would otherwise be lost. If an accepted decision changes, preserve its history with a superseding ADR where appropriate and remove contradictory requirements from the current architecture or build plan.

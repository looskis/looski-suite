# Build plan and acceptance gates

This plan implements the agreed architecture and the first eight applications in [roadmap.md](roadmap.md). No application code or deployment has been validated in this workspace. All milestones below are pending as of October 4, 2026.

Build complete user journeys in small increments. Develop the shared foundation through the first CRM journey instead of spending months building a general platform. Later milestones add optional integrations while retaining standalone application behavior.

## Build readiness and first implementation slice

Ready to begin the foundation and a small Contacts journey. The workspace currently contains planning documents only; no code, installation, or runtime checks have been completed. Remaining product decisions do not block scaffolding and integration probes, but they must be resolved before the behavior that depends on them is finalized.

Recommended first implementation order:

1. Scaffold the Rust workspace and SvelteKit frontend, add the selected license and developer commands, and establish formatting/lint/type-check/test checks. Use the [setup contract](setup-contract.md) for the `looski-suite` namespace, configuration ownership, and proposed ports.
2. Run pinned local PostgreSQL/Supabase Auth with suite migrations in `supabase/migrations/`, applied by the Supabase CLI to an explicit target outside the managed `auth` schema. Gate API/worker startup on successful migrations; validate PGMQ and local ClickHouse packaging. Keep diagnostic ingestion failure independent of application readiness.
3. Implement verified identity, workspace context, and the application authorization interface. Establish bootstrap/login and denial tests before opening business operations to ordinary users; final role presets and visibility policy remain an explicit design task.
4. Implement one workspace-scoped Contacts create/list journey through a focused SQLx DAO and a Rust business operation. Include required audit writes, retry/concurrency behavior, and a durable follow-up probe.
5. Expose that same journey through the HTTP API/OpenAPI, SvelteKit, CLI, and local MCP. Use it to select client/SDK tooling and verify session handling and interface consistency.
6. Expand into shared identifiers, organizations, tags, custom statuses, opportunities, and separate deals using the CRM plan. Validate backup/restore and updates before release.

Resolve RBAC/visibility before implementing production grants, opportunity/deal relationship cardinality before those migrations, and remaining identifier-normalization/merge rules before shipping automatic contact resolution. Set recovery and retention policy before release. Communication connectors, third-party OAuth, a dedicated import product, and the full customer portal are not prerequisites for this first slice; preserve their agreed future integration requirements.

## Milestone 0 — Resolve implementation choices with evidence

Build disposable technical probes only where they answer a listed uncertainty. Record outcomes in [decisions.md](decisions.md) before relying on them in production work.

Docker/Compose for self-hosted Supabase Auth, Axum, SQLx, `utoipa`/`utoipa-axum`, PGMQ, and SvelteKit with TypeScript and SSR are accepted choices. This phase validates their integration and operating requirements. Native runtime packaging and Apple Container are outside the initial build. The [phase 0 and 1 decision checklist](decisions.md#decisions-needed-for-phases-0-and-1) separates remaining proposals from accepted choices.

- Validate the [setup contract](setup-contract.md): two project-scoped installations with distinct bindings/data, health-gated dependencies, one-shot Supabase CLI migration jobs, actionable port conflicts, consistent local/public Auth URLs, and tunnel routing that exposes only the intended gateway paths. Verify migration replay and upgrade against the explicitly selected target; do not add a SQLx migration runner.
- Validate local ClickHouse packaging and a proposed asynchronous collector pipeline. Measure added footprint, test bounded buffering/failure behavior and redaction, and verify business writes do not depend on log-store availability.
- Validate a local Supabase Auth and PostgreSQL bundle based on official Compose configuration and pinned upstream images on Apple Silicon, including service dependencies and Supabase-managed Auth migrations. Keep suite tables outside the standard `auth` schema and validate removal of unused services.
- Exercise first-admin setup, invitations, email/password and magic-link login, callbacks through Cloudflare Tunnel, SMTP, session refresh, logout, and recovery in local and hosted configurations. Verify expired/reused links and prevent implicit signup through ordinary login.
- Define the boundary between verified identity and suite-owned authorization: application services receive actor/workspace context and enforce action/resource permissions independently of cookie or bearer transport. Preserve a place for future verified client/grant restrictions; reject unsupported delegated credentials. OAuth server setup, client registration, consent UI, and delegated grant implementation are deferred beyond phases 0 and 1.
- Verify each installation trusts only its selected Supabase instance. Treat local and hosted instances as independent; do not introduce synchronization or automatic fallback.
- Validate SvelteKit SSR with a CRM table, an editor interaction, session integration, and request-scoped state. Choose supporting UI libraries through this probe; the frontend framework is already selected.
- Validate the selected Axum and SQLx integration, scoped queries, transactions, errors, and migrations; pin compatible release versions.
- Validate OpenAPI generation with the selected `utoipa`/`utoipa-axum`. Select TypeScript client tooling; prove a typed request, success response, error response, security metadata, and contract drift check.
- Probe one authorized read and mutation through the HTTP API, CLI, and MCP using the same Rust application operation. Validate the proposed CLI/stdio-MCP clients, session acquisition/refresh and secure credential storage, workspace selection, and structured errors. Do not make the deferred OAuth server a prerequisite.
- Validate PGMQ extension packaging in local and hosted PostgreSQL; test business changes and enqueue in one SQLx transaction, rollback, retries, lease expiry, crash recovery, and visible exhausted failures.
- Define the transaction interface through a Contacts and CRM operation; prove required changes can commit together without direct cross-module table writes. The detailed booking and form-submission scenario is validated in milestone 8.
- Define the supported Apple Silicon prerequisites, Cloudflare Tunnel hostname/port layout, and first release/update contract. Apply the selected standard AGPLv3 license and notices, verify dependency compatibility and corresponding-source delivery, and finalize the separate trademark policy.

Exit evidence: recorded choices, a runnable infrastructure probe, measured resource usage, and explicit remaining constraints. No claim of five-minute installation until measured on the supported clean-machine setup.

## Milestone 1 — Ship the CRM journey

Implement shared identity, memberships, invitations, authorization, lightweight Contacts, the application registry, and CRM. Add only the files, jobs, audit, and operational capabilities needed for this journey and safe release.

Use the [CRM planning record](crm-plan.md) to refine the reference journey and capability matrix across UI, CLI, API, and MCP. The journey must not force a particular industry or qualification sequence. Custom statuses and tags are required. Gmail, mailbox connectors, and CRM communication implementation are deferred; authentication SMTP and email flows remain in scope. Dedicated bulk-import tooling is deferred in favor of agent-authored scripts using supported CLI/API operations. Before finalizing schema and access behavior, define relationships and coordination scope for the accepted separate opportunity/deal records, decide whether separate lead qualification is needed, and settle email normalization and ambiguous-match/merge policy under the accepted email/E.164 deduplication rule, built-in roles, and whether CRM visibility is workspace-wide or assigned/team-scoped. The decision register contains proposed starting points; these are not silently accepted by the implementation plan. Define file storage, backup retention/recovery expectations, and update recovery before release, using the [local recovery proposal](local-backup-and-recovery.md) as a starting point.

Acceptance:

1. Install the pinned distribution, bootstrap an administrator, enable CRM, and invite a second user with limited access.
2. Load a representative contact dataset using an agent-authored script against supported CLI/API operations; demonstrate per-record errors and safe retries without a dedicated import UI. Create an organization, contact, opportunity, linked deal, directly created deal, and activity. Demonstrate that an opportunity outcome and a deal lifecycle change independently, retaining both histories. Prove workspace-scoped email/E.164 deduplication under concurrent intake, formatting variations, missing fields, shared identifiers, and conflicting email/phone matches; ambiguous cases must not silently merge people.
3. Verify permitted work and denied operations, including access to another workspace and cross-workspace record references.
4. Remove a membership while its login session remains valid; subsequent access fails.
5. Export the supported business records and associated relationships in a documented format.
6. Disable CRM; navigation, direct operations, and its workflows become unavailable while shared Contacts remains usable and CRM data is retained. Re-enable CRM and recover access to that data without replaying old work implicitly.
7. Restart the deployment, back up application/Auth/PGMQ data plus files and recovery configuration, restore into a clean isolated deployment, and repeat the user journey. Reconcile queued external effects before resuming workers; verify backup failure visibility and measure recovery duration.
8. Exercise a supported update and document its recovery procedure.
9. Define and rename a custom status without breaking references or firing a business transition; validate stage changes separately. Confirm contact-only use and direct deal creation remain possible.
10. Demonstrate separate contacts sharing an identifier and review of ambiguous intake; do not silently merge people.
11. Verify correlated local ClickHouse diagnostics and transactional PostgreSQL audit history, including a ClickHouse outage during a valid mutation.
12. Create, rename, attach/detach, and filter tags on contacts, opportunities, and deals through supported interfaces. Verify workspace/resource scope and stable references after renaming.
13. Exercise representative CRM reads and mutations through CLI, API, and MCP. Verify consistent business results, workspace/permission denials, disabled-module behavior, structured errors, and no duplicate mutation on an eligible retry. Document any capability intentionally omitted from an interface.

The first public replacement claim covers only the workflows that pass these checks.

Before advertising local-to-hosted migration, rehearse a transfer to a separate compatible hosted instance: quiesce source writes, transfer supported Auth and application data, preserve or explicitly map identities, verify workspace memberships and record ownership, handle files, configure destination credentials and callbacks, and require fresh sign-in. Verify password login and any enabled identity-provider flows. Document recovery before switching writes to the destination; a connection-string change alone does not pass this gate.

## Milestone 2 — Add forms and CRM intake

Build versioned forms, conditional questions, public submissions, notifications, and export. Public submission rights must be specific to the published form and must not grant general workspace access.

Add a built-in recipe that maps selected answers to a contact and CRM inquiry. Preserve original answers and the form version used. Start with explicit field mapping and documented matching behavior.

Acceptance: forms works with CRM disabled; an enabled recipe produces one intended contact association and inquiry per submission; retries do not duplicate completed actions; ambiguous matching is visible; public input does not overwrite trusted contact data; invalid mapping and missing permissions produce visible failures; a crash after submission commit does not lose the pending workflow.

## Milestone 3 — Add CMS and public lead capture

Implement content types, entries, media, drafts, revisions, and a publishing interface. Provide a representative rendered page to prove content can be served and a published form embedded; a general visual website builder remains in the expansion backlog.

Acceptance: publish and view content using the supported public access mode, submit an embedded form, and observe the permitted CRM result. CMS also works with Forms and CRM disabled. Verify draft visibility, publish permission, file access, and recovery of published content.

## Milestone 4 — Add project delivery

Implement projects, tasks, boards, assignment, deadlines, and comments. Add an optional recipe to create a project from a configured deal milestone.

Acceptance: projects can be created without CRM; retrying the configured deal-to-project recipe does not create a second project for the same configured conversion; project membership and task visibility are enforced independently of CRM access.

## Milestone 5 — Add knowledge base and SOPs

Implement documents, collections, search, revisions, and permissions. Reuse editing implementation where useful without making the knowledge base require the CMS application to be enabled.

Acceptance: members find and read permitted material, restricted material does not leak through search or linked previews, and revision recovery works.

## Milestone 6 — Add customer support

Implement web tickets, assignment, status, internal notes, and customer replies. Define an authenticated or otherwise securely scoped customer reply mechanism before exposing ticket history. Add optional knowledge base links and contact timeline entries.

Acceptance: an external requester can access only their authorized ticket content, internal notes remain private, and the helpdesk works without CRM or Knowledge Base. Treat email intake and delivery as a separately tested integration with an existing provider.

## Milestone 7 — Add time tracking

Implement timers, manual time, billable flags, budgets, and reports, with optional project association.

Acceptance: users can track unassigned work with Projects disabled; timer transitions preserve duration correctly; permitted project totals agree with individual entries; export and access restrictions work.

## Milestone 8 — Add scheduling and the complete intake journey

Implement availability, appointment types, booking pages, basic attendee intake, buffers, cancellation, rescheduling, and reminders. Scheduling works with Forms and CRM disabled. Custom intake uses Forms when enabled; sales follow-up uses CRM when enabled.

Acceptance:

1. Concurrent attempts to reserve the same exclusive slot cannot both succeed. Invalid intake does not leave a confirmed appointment.
2. With a custom intake form, commit the appointment, versioned submission, association, and durable booking event together.
3. Trigger the booking intake recipe once from the confirmed booking. The embedded form submission must not independently trigger the same CRM ingestion.
4. Resolve the contact, apply approved mappings, record the inquiry, and show the appointment and answers in an authorized contact timeline.
5. Restart the worker between steps and retry delivery; the intended inquiry and completed effects are not duplicated.
6. If CRM processing fails, retain the confirmed booking and show the failed workflow to an authorized administrator with a safe retry path.
7. Rescheduling preserves the appointment association; cancellation records the change without deleting the contact. Obsolete reminders do not fire.
8. Test time zones, daylight-saving transitions, buffers, and changes to availability. Validate calendar provider synchronization separately before promising avoidance of conflicts with external calendars.
9. Disable a participating application during pending work; that work becomes visibly paused or blocked and cannot bypass the disabled state.

This milestone proves the flagship journey: a customer books through a form, becomes associated with the right contact and sales inquiry, and appears on one coherent timeline.

## Future external OAuth release gate

External developer OAuth support is a future capability, not a phase 0 or 1 deliverable. When scheduled, validate the selected provider and pinned local/hosted versions using a separate client, PKCE, consent, bearer-authenticated Axum requests, and refresh. Implement application-owned grants with explicit workspace/action limits. Before advertising support, demonstrate refusal of ungranted operations, cross-workspace requests, revoked grants, ID tokens, disabled-module requests, and attempts to regain revoked access through refresh. Finalize client registration, consent binding, rate limits, and developer documentation then.

## Release gates for every application

| Area | Required evidence |
| --- | --- |
| Business usefulness | A complete representative journey and a precise list of unsupported workflows. |
| Interfaces | Supported business capability matrix across CLI, API, and MCP; shared business rules, representative contract/denial tests, and documented exceptions. |
| Module selection | Works with optional applications disabled; direct requests and background work respect enabled state. |
| Access | Positive and negative authorization checks across users, resources, and workspaces, including files, exports, search, and timelines. |
| Data | Tested constraints, migrations, documented import/export behavior, and no unintended cross-workspace associations. |
| Background work | Atomic scheduling, idempotency where supported, visible failures, bounded retries, crash recovery, and explicit handling of uncertain external outcomes. |
| Operations | Pinned Apple Silicon-compatible release, health/status checks, restart, supported update, backup and verified restore of database and files. |
| Performance | Measured whole-stack footprint and representative latency/import/queue behavior; publish measurements with dataset and hardware context. |
| Public claims | Separate demonstrated results from goals; disclose reused foundation and remaining external providers. |

Use domain tests, real-Postgres integration tests, and complete journey tests according to the behavior under review. Mocked-only checks do not establish authorization, transaction, migration, or job recovery correctness.

## Delivery discipline

Every implementation task should identify its milestone, owning module, user-visible outcome, and acceptance evidence. Update these documents when a decision changes; do not turn a provisional library recommendation into an accepted choice implicitly through code.

Do not start the general automation builder, custom database platform, or internal tool builder as prerequisites for this plan. Built-in recipes and focused module interfaces are enough to establish the suite's composition model.

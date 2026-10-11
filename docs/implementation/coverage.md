# Build coverage

This is the implementation checklist for the October 12 build request. Accepted requirements remain in the CRM plan, architecture, schema design, and build plan. An unchecked item is not shipped or verified. Deferred roadmap features remain deferred unless scheduled explicitly.

## Foundation and first CRM release

- [x] Local Supabase, migration runner, Rust API, SvelteKit SSR, gateway.
- [x] Owner provisioning, password login, session refresh/logout, workspace discovery across interfaces.
- [x] Contacts create/read/update with identifier validation, reviewed sharing, and concurrent matching safeguards.
- [x] Organizations and temporal relationships, primary selection, preserved merge/archive history.
- [x] Automatic intake resolution and persistent review.
- [x] Scoped action authorization for workspace/assigned Contacts operations.
- [x] Supported staff role/member administration and final-administrator protection.
- [x] Optional CRM module selection, disable/re-enable preserving shared Contacts and CRM records.
- [x] Staff invitations, explicit recipient consent and queued Auth email; team policies remain unavailable.
- [x] Transactional audit, request replay, optimistic edits, archive/restore, merge/aliases.
- [x] Controlled archived-contact permanent deletion, scoped confirmation, retained-reference policy and replay redaction.
- [x] Shared tag catalog and contact/organization assignments, filters, lifecycle and merge preservation.
- [x] Opportunity pipelines/stages, deals/categories/statuses and independent histories across API/CLI/MCP/web.
- [x] CRM tag assignments and filters with shared catalog lifecycle.
- [x] CRM participants.
- [x] Shared Tasks with independent assignment, lifecycle, history and four-interface parity.
- [x] Shared task types, immutable template revisions and pinned instantiation with current assignment authority.
- [x] CRM task links and atomic follow-up composition with independent Tasks scope.
- [x] Configurable activity types and logged activities with typed anchors, scoped history and four-interface parity.
- [x] Pinned coordination checklist templates and atomic deal instances with scoped task projection.
- [x] Bounded deal-status rules and pinned queued coordination.
- [x] Explicit removable starter package, copied editable definitions and retained business history.
- [x] Combined timeline with independent source visibility and bounded cursor pagination.
- [x] Contacts API/OpenAPI, CLI, MCP, web journeys and pagination.
- [x] Implemented CRM interface parity and versioned, scoped business JSON export with an atomic all-page collector.
- [x] Durable queued follow-up, retries, leases, exhaustion and explicit module disable/resume.
- [x] Local magic links/recovery, private test mail, SMTP configuration and callback/outage behavior.
- [ ] External SMTP deliverability, public Cloudflare callback and hosted-mode validation.
- [x] Shared scoped Files library, immutable byte storage, browser/API/CLI/MCP, metadata export and transfer limits.
- [x] Encrypted coordinated backups, offline image portability, isolated restored writes and partial-update recovery.
- [x] ClickHouse diagnostic ingestion, redaction and outage behavior.
- [x] Diagnostic resource footprint and complete application regression; recovery checks are tracked separately above.
- [x] Local full journey, denial/concurrency tests, production build, standards/spec review.
- [x] CLI/MCP integrated browser login (PKCE, pasted code), Keychain/private-file storage, locked refresh and logout; externally managed token files remain supported. See [CLI login](cli-login-slice.md).
- [ ] Public distribution: corresponding-source delivery, complete dependency notices and image/OS advisory review.

## Subsequent application milestones

- [x] Forms and versioned submissions/intake. See [Forms](forms-slice.md).
- [x] CMS drafts/revisions/media/publishing and public pages with embedded forms. See [CMS](cms-slice.md).
- [x] Projects and boards over shared Tasks. See [Projects](projects-slice.md).
- [x] Knowledge base and SOPs. See [Knowledge base](knowledge-slice.md).
- [x] Helpdesk and restricted customer replies. See [Helpdesk](helpdesk-slice.md): upgraded/fresh-install API and browser journeys, real Chrome, full application regression and isolated recovery passed.
- [x] Time tracking: manual entries, durable timers, billable totals, budgets, optional Projects, scoped export and all staff interfaces. See [milestones 7–8](time-scheduling-slice.md).
- [x] Scheduling and atomic booking/form intake with durable CRM follow-up: concurrency/DST/buffers, reminder fencing, browser/API/CLI/MCP, upgraded and fresh installs, full sequential regression and isolated recovery passed. See [milestones 7–8](time-scheduling-slice.md).

## Verification boundaries

Use the approved HTTP business contract, CLI/MCP clients, browser journeys, and real PostgreSQL constraints/transactions. Tests establish observable outcomes, denial, rollback, and concurrency behavior. Do not substitute snapshots of implementation details for those guarantees. Review against baseline `8a82e86` and the accepted specs, preserving preexisting research files.

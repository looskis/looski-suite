# Build coverage

This is the implementation checklist for the October 12 build request. Accepted requirements remain in the CRM plan, architecture, schema design, and build plan. An unchecked item is not shipped or verified. Deferred roadmap features remain deferred unless scheduled explicitly.

## Foundation and first CRM release

- [x] Local Supabase, migration runner, Rust API, SvelteKit SSR, gateway.
- [x] Owner provisioning, password login, session refresh/logout, workspace discovery across interfaces.
- [x] Contacts create/read/update with identifier validation, reviewed sharing, and concurrent matching safeguards.
- [ ] Organizations and relationships, automatic intake resolution and persistent review.
- [x] Scoped action authorization for workspace/assigned Contacts operations.
- [ ] Role/member administration, invitations, teams, and module selection.
- [x] Transactional audit, request replay, optimistic edits, archive/restore, merge/aliases.
- [ ] Controlled permanent deletion and detailed reference/erase policy.
- [ ] Tags, opportunity pipelines/stages, deals/categories/statuses, participants, independent histories.
- [ ] Shared Tasks, activities, coordination configuration/templates, safe template removal.
- [x] Contacts API/OpenAPI, CLI, MCP, web journeys and pagination.
- [ ] Broader CRM interface parity and export.
- [ ] Durable queued follow-up, retries, leases, exhaustion and module disable/resume.
- [ ] Magic links/recovery, SMTP, public callback and hosted-mode validation.
- [ ] Files, encrypted coordinated backups, isolated restore and update recovery.
- [ ] ClickHouse diagnostic ingestion, redaction, outage behavior and footprint measurements.
- [ ] Full journey, denial/concurrency tests, production build, standards/spec review.

## Subsequent application milestones

- [ ] Forms and versioned submissions/intake.
- [ ] CMS drafts/revisions/publishing.
- [ ] Projects and boards over shared Tasks.
- [ ] Knowledge base and SOPs.
- [ ] Helpdesk and restricted customer replies.
- [ ] Time tracking.
- [ ] Scheduling and atomic booking/form intake with durable CRM follow-up.

## Verification boundaries

Use the approved HTTP business contract, CLI/MCP clients, browser journeys, and real PostgreSQL constraints/transactions. Tests establish observable outcomes, denial, rollback, and concurrency behavior. Do not substitute snapshots of implementation details for those guarantees. Review against baseline `8a82e86` and the accepted specs, preserving preexisting research files.

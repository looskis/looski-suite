# Contacts implementation slice

October 12, 2026. This records the first Contacts increment, not completion of CRM or the release plan. The broader checklist is [coverage](coverage.md).

## Operations and boundaries

Workspace-scoped list/read, explicit creation, replacement of editable fields, archive/restore, and confirmed duplicate merge run as Rust business operations with focused Contacts DAOs. Foundation-owned authorization, audit, and request replay accept the same PostgreSQL transaction. API handlers verify Supabase identity; CLI/MCP use that HTTP boundary; SvelteKit uses server-side session cookies and forwards bearer credentials internally.

Each action checks current user, identity mapping, workspace, staff membership, active role, action grant, and record scope. Implemented scopes are workspace and assigned. Team and external access are not enabled. Scope union applies only to grants for the same action. Creation defaults ownership to the caller. Membership in a workspace alone grants no action.

The migration upgrades existing owner roles with an explicit Contacts permission list. New owner bootstrap grants that same list. No wildcard future permissions are granted. Role administration and invitations remain subsequent work.

## Matching and lifecycle

A case-sensitive identifier registry stores one workspace/kind/canonical-value row with independently retained contact associations. Shared identifiers require an explicit reviewed-sharing flag and separately scoped permission. Email domain case is normalized; local-part case, dots, and plus tags are retained. Phone parsing uses pinned `phonenumber` metadata and explicit country context, produces E.164, and retains extensions on associations. Neither matching nor merge creates or changes an Auth identity.

Explicit creation encountering existing identifiers returns a review conflict rather than guessing identity. This includes archived matches and hidden contacts; candidate IDs are not disclosed. Automatic intake resolution and a persisted review inbox are still pending. Identifier locks are acquired in deterministic order, including keys not yet inserted.

Archive retains data and identifiers. Merge requires both record versions, a chosen survivor, and resolved fields; it preserves identifiers and source records/notes, records an audit event, and flattens existing aliases. Reads through an old ID separately authorize the survivor. The supported bound is 20 identifiers per contact; an oversized merge fails atomically instead of creating an uneditable survivor. There is no promised unmerge or permanent-delete operation in this increment.

## Programmatic contract

Paths start at `/api/v1/workspaces/{workspace_id}/contacts`. Reads use bounded cursor pagination (default 50, maximum 100), active/archived filtering, and name search. A full page contains only permitted records. Detail reads and writes enforce the same action-specific scope.

Mutations require `Idempotency-Key`; edits, archive/restore, and merge require expected versions. Replay is scoped to workspace, actor, operation, and key. It returns the original saved result after checking current access. Changed input conflicts. Keys expire after seven days and remain retained: an expired key requires explicit reconciliation instead of silently re-executing. There is no automatic business-record purge.

Errors have stable codes and safe messages. `X-Request-Id` is server-generated and correlates transactional audit and diagnostics. CLI errors include status/code/message/request ID as JSON on stderr; exit codes are 3 for invalid/not-found, 4 for unauthorized/forbidden, 5 for conflicts, 6 for server unavailability, and 1 for other runtime failures. MCP business errors retain the same structured fields. Interface attribution (`web`, `api`, `cli`, `mcp`, `operator`) is bounded metadata, never authority or proof of client identity.

## Verification

The real-installation suite is `scripts/contacts_smoke.py`; it refuses non-fixture project names. It exercises HTTP and server-rendered browser forms, CLI and MCP reads/writes, scope denials, case/phone normalization, request replay and collisions, same-key and distinct-key races, stale edits, merge/history/aliases, archive/restore, rollback on audit failure, request correlation, and origin protection. Fixtures are retained in the isolated installation, never inserted into the primary workspace.

Run generated contract/type checks, Rust lint/tests, setup guard tests, production web build, and the identity regression suite alongside the Contacts suite. Review baseline for this increment is `8a82e86`. Both review axes found issues that were fixed before final verification: merge bounds, shared persistence ownership, interface attribution, extractor errors, distinct-key concurrency coverage, and CLI/MCP request correlation.

Remaining: organizations/relationships, automatic intake/review, tags, export, permanent deletion, role/member administration, CRM records, Tasks, durable follow-up, and operational release gates. See the canonical build plan; this document does not reduce that scope.

### Verified checkpoint

The complete Contacts acceptance suite passed on `looski-suite-identity-check` at port 58890. `make check test` passed (Rust formatting/lint, frontend formatting/types, generated contract drift, five Rust tests, four setup tests). The production web build passed. Identity regressions passed separately because their revocation probes mutate the same fixture authority state. A fresh installation (`looski-contacts-identity-check`, port 58892) also passed the complete migration history, first-owner rollback/recovery/concurrent-bootstrap checks, identity regression, and Contacts acceptance.

### Standards review

Three findings: merge output bound, foundation persistence ownership, and source-interface attribution. All were fixed; the standards reviewer rechecked and reported no new blocking defect. Review was read-only; execution evidence comes from the test suite.

### Spec review

Four findings across initial review and recheck: merge output bound, extractor error consistency, distinct-key identifier race coverage, and CLI/MCP request correlation. All were fixed and covered by the passing acceptance run. Deferred roadmap work remains explicitly unchecked in the coverage record.

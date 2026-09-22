# Tags checkpoint

October 12, 2026; schema version 7. The foundation owns workspace tag definitions. Contacts owns separate contact-tag and organization-tag assignment tables with workspace-qualified foreign keys. CRM assignments will ship with CRM records.

## Behavior and contract

Create, search/page, read, rename, color, archive and restore tags through web/API/CLI/MCP. Names are trimmed and keyed using Rust Unicode lowercase; names remain reserved while archived. This is not full Unicode case folding. Concurrent creates for one key serialize and return a conflict for the loser. Tags have stable IDs and optimistic versions. A rename retains assignments; omitted archive state preserves it, and restoration requires explicit `archived: false`. Colors are optional six-digit hex values.

Catalog permissions require workspace scope. Assignment reads require current parent visibility and catalog-read authority; writes additionally require the corresponding Contacts or Organizations tag-management permission. A broad read grant cannot enlarge an assigned mutation grant. Tag filtering occurs inside the scoped parent query before pagination. Tags never grant access to records.

Assignment replacement takes a parent version and up to 50 distinct tag IDs. Parent and tag locks serialize edits and archive changes. Existing archived assignments can be retained or detached; new assignments require active parents and tags. Contact merge unions tags, preserves source history, and rejects a result exceeding 50 assignments atomically. Alias reads reauthorize the survivor. Merge previews display authorized tags.

Every mutation commits its audit and retry result together with changes. Retries recheck current access before returning the original response. The browser selection panel preserves existing tags outside the current catalog search results. The API catalog query has only supported fields; parent lists additionally accept `tag_id`.

## Evidence and limits

`scripts/tags_smoke.py` verifies concurrent names and parent writes, stable rename references, archive/explicit restore, detachment, assigned scopes and filters, merge/aliases, cross-workspace constraints, all four interfaces, audit-failure rollback and merge capacity. The entire migration history, new-owner bootstrap grants, identity flows, Organizations and Tags journeys also passed on a fresh `looski-tags-identity-check` installation. Its volumes and private fixture were retained after stopping the stack. The archive regression was observed failing on the original image and passing after correction. `make check test` passes; generated OpenAPI/Svelte types match. The two-axis review findings (implicit restoration and an ignored advertised filter) were fixed and rechecked.

There is no tag hard-delete operation or automatic purge. Historical assignments are represented by retained merge source links and transactional audit events; a public history/export view remains pending. General CRM, intake and operational release gates are tracked separately in [coverage](coverage.md).

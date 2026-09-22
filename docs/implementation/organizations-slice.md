# Organizations checkpoint

October 12, 2026; schema version 6. Organizations belong to shared Contacts and work independently of CRM. This checkpoint extends, and does not complete, milestone 1.

## Behavior

Create, search/page, read, edit, archive and restore organizations through web/API/CLI/MCP. A name or website domain is not a unique identity. Names, website URLs and notes are validated; owner must be an active workspace member. Workspace/assigned permission grants are checked per action before pagination and linked projections.

A contact has up to 100 current organization relationships, each with a label, job title and optional primary flag. At most one is primary. Changes end old rows and insert new versions; ending a relationship retains its history. Changes require the expected contact version. Choosing a primary demotes the previous primary while retaining that earlier version. Both parent rows are locked while changing relationships, serializing contact edits and organization archive changes.

Contact merge previews display authorized relationships. Conflicting labels, titles or distinct primary organizations must be resolved first; successful merges combine current relationships and retain previous source/survivor rows. Old contact URLs redirect to the editable survivor; old-ID relationship reads resolve the survivor subject to current access. Archive retains links and blocks new ones until restored. Permanent deletion is unavailable.

Every mutation uses the same transaction for business changes, audit and retry result. Retries recheck current permission and record visibility before returning the original result. Organizations and relationships have workspace-qualified foreign keys and explicit scoped permissions. Only implemented owner grants are seeded; no future wildcard permissions.

## Evidence

`scripts/organizations_smoke.py` covers create/edit/archive/restore, same-name organizations, validation, stale edits, primary concurrency, relationship history, merge conflicts/aliases, assigned visibility, cross-workspace constraints, retry authority and web/API/CLI/MCP parity. `scripts/contacts_smoke.py` supplies regression coverage for identifier matching, shared identifiers, merge/audit rollback, scopes and retries.

The two-axis review found one remaining issue: the merge preview omitted relationship details. That preview now fetches authorized links and offers navigation to resolve conflicts. Spec review found no further defects within this increment. The wider build checklist remains in [coverage](coverage.md).

## Limits

Historical relationship rows are retained in PostgreSQL; a public history browser/export is pending. Organization merge and reliable organization identity resolution are not advertised. The current API timestamp representation follows the existing Contacts response; a standardized timestamp contract remains a release task.

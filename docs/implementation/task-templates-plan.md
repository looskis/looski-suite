# Shared task configuration increment

Tasks owns workspace task types and reusable task templates independently of CRM. A type has stable identity, editable label/order and archival. Existing tasks retain the saved type label in history. Retiring a type denies new assignment but does not prevent editing an existing task that retains it. Type changes cannot alter task lifecycle behavior; open/in-progress/completed/cancelled remains the shared actionable-work contract.

A task template has stable identity and immutable numbered revisions. Each revision records a task title/body, optional type, a bounded due-date offset in calendar days, and an explicit assignment rule: the initiating member, a context owner supplied by the composing application, or a fixed membership. Require an active fixed membership at configuration and revalidate on use. Templates do not grant permissions or carry arbitrary executable code.

Template edits produce a new revision without changing existing tasks. Archival prevents new use but preserves revisions and instances. Configuration read/manage actions are workspace-scoped; applying a template independently enforces Tasks create/read and permitted active assignment. The caller chooses an explicit base date. Contextual assignment cannot override current scope, and a missing contextual owner is an input error rather than an implicit fallback.

CRM coordination will reference pinned Tasks revision IDs through Tasks-owned providers. Tasks creates the resulting task inside the caller's SQLx transaction without committing it. The caller owns the outer retry contract, and Tasks owns validation, assignment, type resolution, history and audit. Applying a pinned older revision is explicit; neither a newer revision nor a renamed type silently changes its recorded work specification.

Expose types/templates and revision history through API/OpenAPI, CLI, MCP and SSR. Verify independent module availability, immutable revisions, retired definitions, assignment/date boundaries, rollback, retained drafts and concurrency before composition relies on them.

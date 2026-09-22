# Shared task configuration checkpoint

Schema 18 adds task types, template heads and immutable numbered revisions, plus typed task provenance. Tasks remains independent of CRM. Configuration requires separate workspace-wide read/manage permissions; creation from a template additionally requires current Tasks create/read authority and permitted active assignment.

Types have stable IDs, configurable order and archival. Task edits preserve an omitted type and clear an explicit null; new assignment to a retired type is rejected, while existing assignment may be retained. Saved task/history labels do not change when a type is renamed. Ordinary edits cannot inject or change source-template provenance.

Task specifications pin title/notes, type and label, assignment rule, fixed membership when applicable, and a bounded calendar-day offset. Material edits publish new immutable revisions; metadata changes retain the revision. A deferred composite foreign key ensures the template head always points to an existing revision at commit. History is append-only for the runtime role.

Instantiation chooses an explicit revision and base date. It resolves initiating/contextual/fixed responsibility, validates the resulting calendar date and active type/member, and creates an ordinary task with its source revision in the same transaction as audit/history/replay. Old revision type labels remain pinned even after catalog rename. Retiring a template prevents new work but preserves its instances and original-result retries subject to current permissions.

API/OpenAPI, CLI, MCP and SSR cover configuration, revision history and instantiation. Web forms preserve invalid/conflicting fields, versions and keys. The initial isolated journey passed immutable revisions, type omission/clearing, archival, assignment/date boundaries, atomic rollback, independent grants/module state, workspace FKs, concurrent publication/creation and all interfaces. Final static checks/unit tests, primary gateway smoke and both full fresh-install/upgraded-database regressions pass, including the pinned-label refinement. Review found no implementation defects; the requested pinning assertion now covers both the new task and its first history snapshot.

Coordination checklist composition, triggers, queued execution, combined timelines and removable starter packages remain separate release gates.

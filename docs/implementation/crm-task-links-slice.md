# CRM task links checkpoint

Schema 15 adds typed opportunity/deal task links with workspace-qualified foreign keys, durable IDs, end/restore state and immutable link history. The shared task remains authoritative for assignment, due date, content and lifecycle; changing a link never edits its task.

Every operation requires enabled CRM and independent parent scope. Mutations also need the relevant task-link management grant and independent Tasks read authority. Creating a follow-up additionally requires Tasks create/read and permitted active assignment. Current authority is rechecked on original-result retry, including a task's current assignee. Missing task read produces an empty link list and unavailable link history; neither hidden cursors nor pagination reveal omitted task IDs.

Tasks owns a bounded authorized projection interface. CRM reads candidate links in batches of 100, obtains independently scoped task projections in one batch, and fills public pages only from visible results. No per-row Tasks permission/query loop is needed. Activation requires active parent and task; ending retains history and is allowed on archived records.

CRM creates a follow-up by invoking a Tasks-owned transaction operation. Task, task history/audit, CRM link/history/audit, parent version and retry result commit atomically. The provider does not commit the caller's transaction. Template/job-driven work will use the same composition boundary with its separately established authority.

Fresh-install and upgraded-database full acceptance pass both parent types across API, CLI, MCP and SSR, including failed link insertion after task creation (no orphan task/audit), independent scopes and replay, hidden batches, archives, real foreign-workspace references, module disablement, off-page draft selection and stale link/end requests. Static checks/unit tests and primary gateway smoke pass. Both full regressions include final batched projections and simultaneous retry/conflict cases. Coordination templates, activities and durable jobs remain outstanding.

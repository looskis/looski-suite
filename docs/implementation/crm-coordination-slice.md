# CRM checklist composition checkpoint

Schema 19 adds workspace-owned coordination templates, immutable ordered revisions and retained deal checklist instances. A revision contains 1–25 named steps referencing pinned shared Tasks template revisions. Material item edits publish a new revision; renaming or archiving the template preserves its revision and every existing instance.

Applying a revision requires independently scoped deal read, `crm.deals.coordination.apply`, task-link management, CRM configuration read, Tasks configuration read and Tasks create/read authority. The caller supplies the deal's expected version and a base calendar date. Contextual assignment uses the current deal owner; Tasks validates active assignments and date arithmetic. Each new typed link increments the parent version.

Task creation, task/link history, checklist instance/items, audit and original-result replay commit together. Mid-composition or later-step failure leaves none of these partial writes. Concurrent retries return one original application; conflicting independent applications cannot create orphan tasks. Replay rechecks current authority for the parent and every task in its saved response. The result storage bound is 2 MiB for up to 25 full task snapshots, while request storage remains limited to 64 KiB.

Instance lists contain bounded summary headers. Detail views project only independently readable tasks without hidden task identifiers or aggregate counts. Task completion/archive remains Tasks-owned. Ending a CRM link leaves its task and checklist evidence intact and is shown explicitly. Archived parents/templates prevent new work, while existing authorized history remains readable.

HTTP/OpenAPI, CLI, MCP and SSR expose definition creation/edit/archive, revision history, instance application/list/detail. Forms preserve rejected fields, ordered repeated controls, expected versions and retry keys. The isolated journey passed immutable revisions, archive retention, independently scoped grants, module disablement, hidden projections, atomic rollback, composite constraints, concurrent application and maximum-size replay. Standards and spec reviews found no actionable findings. Full fresh-install and upgraded-database acceptance suites, static checks/unit tests and the primary gateway smoke all passed.

Status triggers, durable queue execution, combined timelines and starter packages remain separate unfinished release gates.

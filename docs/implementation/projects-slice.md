# Projects checkpoint

November 1, 2026; schema version 32. Projects is the optional application from Milestone 4: project grouping, membership, boards, delivery views and comments over shared Tasks, plus an optional built-in recipe that creates a project from a deal milestone. It works with CRM disabled.

## Behavior and contract

Staff manage projects through web, API, CLI and MCP. A project has a name, description, status (active, on hold, completed, cancelled), optional deadline and archive state, with optimistic versions, idempotent retries and transactional audit. Its creator joins as a member. Archiving freezes the board and comments and leaves its tasks alone.

There is no task table here. A card is a typed, workspace-qualified link to one shared task; title, notes, assignee, deadline, status and history stay in Tasks. Projects reaches them only through the Tasks interface: its SQL names no `tasks` or `crm` table, and the schema refers to them only through foreign keys. A task is on at most one board at a time, and may also have CRM links.

### Boards

A new project has three columns: To do, In progress, Done. Columns can be renamed, reordered, added (at most 20) and removed. Removing one names a destination that always receives its cards, so the request reveals nothing about hidden cards, and those cards keep their task status.

A column may name a task status. Moving a card into it asks Tasks for that status under the mover's own authority; if Tasks refuses, the whole move rolls back. A move inside a column, a removal, or a link of an existing task changes no status. New tasks take the status of the column they are created in. Changing a task's status in Tasks does not move its card, since Tasks knows nothing of Projects. A board holds at most 500 tasks.

Moves name a target column and the card to sit before, or none for last. Each runs under the workspace lock taken by every Tasks and Projects mutation and renumbers whole columns, so places stay dense and unique; a deferred unique constraint on column and position backs this at commit. Concurrent moves of different cards all apply in some order. A move of a card someone else just moved fails on the card's version and says to reload, rather than undoing theirs. A retried key applies once and returns the original answer.

The web board moves cards with plain forms (up, down, to a column), so it works without scripting. Dragging fills in the same form.

### Assignment and deadlines

`tasks::plan_in_transaction` is the new Tasks interface: change assignee, deadline or status under the caller's transaction, with Tasks establishing authority, scope, archive state and assignment rules. On top of it Projects adds one rule: a new assignee chosen through a project must be a project member. Tasks may still assign a task to anyone, because it knows nothing of Projects; the board then flags the assignee as outside the project rather than hiding the card. Project deadlines are not enforced on tasks; the delivery view reports tasks due after them.

### Delivery views

The board; a project's delivery view (progress, overdue, due within seven days, no deadline, after the project's deadline, workload by assignee, and tasks soonest deadline first with status, deadline and assignee filters); and **My work**, the caller's tasks across projects they may read. Dates compare with the installation's current date. Every count covers only tasks the reader can see.

### Permissions

| Permission | Grants | Scopes |
| --- | --- | --- |
| `projects.read` | Projects, members, columns, boards, delivery views, comments | workspace, assigned |
| `projects.create` | Create projects | workspace, assigned |
| `projects.manage` | Edit, archive and restore; manage columns | workspace, assigned |
| `projects.members.manage` | Add and remove members; list the workspace's active members to choose from | workspace, assigned |
| `projects.board.edit` | Add, remove, move and schedule cards | workspace, assigned |
| `projects.comments.write` | Comment; edit and remove one's own | workspace, assigned |
| `projects.comments.moderate` | Remove anyone's comment | workspace, assigned |
| `projects.recipes.manage` | Configure, activate and retry the recipe | workspace |

Assigned scope means projects the member belongs to; a project outside it answers 404. Membership grants nothing without a role, and removal takes effect on the next request with the same token.

Project and task visibility are independent. A card, its comments and its place in any count need `projects.read` on the project **and** Tasks read of that task under Tasks' own scope. A member with assigned `tasks.read` sees their own cards; positions count only what they can see, so no gap reveals a hidden card, and a hidden card as a move target answers like one that does not exist. **Seeing teammates' cards therefore needs workspace-wide `tasks.read`.** One exception is deliberate and small: linking a visible task already on another board answers that it is on a board, without saying which.

CRM and Projects reveal nothing to each other. Cards carry no CRM link, CRM task links carry no project, and a project's `source_deal_id` is present only for readers who can read that deal right now, with CRM enabled.

### Comments and stored markup

Comments belong to a card and use the CMS rich text subset through its pure parser, with no dependency on CMS being enabled. The source is plain text; the server parses it into typed blocks, HTML in it stays literal, and an unsafe link is refused when saved and renders as plain text if it reached storage another way. The web view renders only those blocks through the same typed component as CMS, with no raw HTML output. Task titles, project names and descriptions are escaped text.

Authors edit their own comments. Removal, by the author or a moderator, erases the text and the retry payloads that held it, and leaves a marker in the thread. Audit records never hold comment text. Comments survive a card's removal and return if the task is added back.

### Deal-to-project recipe

An administrator names a deal status as the milestone, an optional project name prefix, members, and whether the deal's owner joins. Recipes start inactive; saving appends an immutable revision and each run keeps its own. One recipe may be active per status. Activation requires every permission a run uses, workspace-wide (`projects.recipes.manage`, `projects.read`, `projects.create`, `projects.members.manage`, `crm.deals.read`), and records the activator, whose **current** permissions each run re-checks.

When a deal reaches that status, or is created in it, CRM calls `projects::recipes::deal_reached_status` in the same transaction, which commits one conversion and its durable job. `UNIQUE(workspace, recipe, deal)` makes that the only conversion ever: re-entering the status finds it and does nothing. The worker reads the deal through CRM and creates the project, its members and the conversion's completion in the transaction that finishes the job, so a retry after any failure starts clean and a completed conversion is never redone. The project name is a copy of the deal title, chosen by the administrator's configuration; no deal link is shown to members without CRM access.

Failures are job states shown on the recipe page as **Failed** with their reason: `module_disabled`, `authority`, `invalid_input` (a listed member left), `unavailable` (the deal was archived), or exhausted retries. Retrying resumes the same conversion. A milestone reached while Projects is off still commits with CRM's change and shows as blocked once Projects is back. Disabling CRM or Projects blocks pending work, and re-enabling never replays it implicitly. Deactivating stops new conversions only. A deal that leaves the status before its job runs still converts, since the milestone was reached; a cancelled job leaves its conversion in place, so that deal does not convert again under that recipe.

### Module, export and recovery

Disabling Projects closes every route, including export, CLI and MCP, with `403`, and blocks pending conversions. Every record is retained, shared Tasks keeps working on the same tasks, and re-enabling restores the board with current task state. Export adds `projects`, `project_members`, `project_columns`, `project_cards` and `project_comments`; see [business export](../business-export.md). Recovery needs no specific step; see [recovery](../recovery.md).

## Evidence and limits

`scripts/projects_smoke.py` runs on real Postgres with real signed-in second users and with CRM disabled until its CRM sections. It covers validation, retries and stale versions; cards over shared Tasks and status-mapped moves; the scheduling rules; column management; concurrent moves of different cards, of one card, and of one retry, checked against dense unique positions and the constraint itself; live membership revocation; hidden tasks with no leaked counts, IDs or places; rollback of a move Tasks refuses; delivery figures, filters and paging; comment storage, refused and smuggled links, author and moderator rules, and erased text; archive; cross-workspace and cross-project references; export pages and authority; CLI and MCP parity; disabling Projects with Tasks still usable; a task on both a deal and a board seen from each side; and the recipe through retries, status re-entry, job resume, concurrency, an injected fault, a stopped worker, an expired lease, disabled modules before and during pending work, revoked authority, an invalid configuration and removed membership.

`scripts/projects_browser_smoke.py` drives real HTTP through the gateway with a scripting-free client: application selection, a rejected creation keeping its draft, adding and linking tasks, up, down and column moves, a stale move explaining itself, forged origins, scheduling, inert comment markup, a refused comment keeping its draft, editing and removal, delivery filters, members, columns, archive, the recipe with a visible failure and its retry, session gates and framing headers on every page, and disabled pages. Both run in `scripts/acceptance.py`.

A one-off headless Chrome run on November 1, 2026 confirmed what those clients cannot: with `onerror`, `<script>` and `<svg onload>` payloads in a project name, description, task title and comment, no payload ran, no dialog opened, no element was injected and the markup showed as text; a real form post from the page passed the origin check; and synthesized drag events moved a card before another and into another column through the page's own handlers. This check is not automated.

`scripts/recovery_smoke.py` now seeds a board with a comment and checks order, tasks and comment after an isolated restore. The recovery scripts accept schema 32, and the full rehearsal passed on this code on November 1, 2026. It is not part of `acceptance.py`.

Not supported: project templates or starter tasks from a recipe; a project's own discussion outside a card; attachments, mentions, notifications or comment edit history; dependencies, milestones, sprints, Gantt or calendar views, swimlanes or WIP limits; roll-ups across projects beyond My work; a task on several boards; a card following a status changed in Tasks; widening Tasks visibility through project membership; time zones per user (dates compare with the installation's date); a board above 500 tasks; external or customer access; and recipe triggers other than a deal status. Recipe configuration and run history are not in the business export. The member directory is capped at 500 active members. No separate footprint or latency measurement was taken; Projects adds no service or background worker beyond one job kind.

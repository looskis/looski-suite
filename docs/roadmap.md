# Application roadmap

This is the accepted build order as of October 4, 2026. Applications 1–8 form the initial roadmap; applications 9–25 are an ordered expansion backlog to revisit using evidence from actual users. This is a sequence, not a release schedule or a promise of one production-ready application per day.

The order favors business value, a bounded replacement scope, reuse across the suite, and limited dependence on external networks or proprietary data. Reference products identify workflows to study, not a commitment to reproduce an entire vendor's offering. Product scope and exclusions live in [intent.md](../intent.md); implementation milestones live in the [build plan](build-plan.md).

## Initial roadmap

| Order | Application | Reference products | First replacement scope | Optional connections |
| --- | --- | --- | --- | --- |
| 1 | CRM | Pipedrive, HubSpot CRM | Contacts, organizations, sales opportunities and separate live/historical deals, configurable stages, tags, activities, shared follow-up tasks, and scriptable data access/export. Gmail and communication connectors are deferred. One opportunity can produce many deals, each with an optional single origin; qualification uses opportunity stages. Configurable coordination behavior and removable starter templates need detailed design; dedicated import tooling is deferred. | Uses the shared Contacts module; supplies sales context to later applications. |
| 2 | Forms and surveys | Typeform, Jotform | Builder, conditional questions, embeds, versioned submissions, notifications, export. | Create CRM inquiries and link source answers. |
| 3 | CMS | Contentful, Sanity | Content types, entries, media, drafts, revisions, publishing interface. | Embed forms in published content. |
| 4 | Projects and tasks | Trello, Asana, Basecamp | Projects, boards, delivery views over shared Tasks, assignment, deadlines, comments. | Create a project from a configured deal milestone. |
| 5 | Knowledge base and SOPs | Confluence, Notion wiki workflows | Documents, collections, search, revisions, permissions. | Reuse content editing capabilities; connect articles to support. |
| 6 | Helpdesk | Zendesk, Help Scout | Web tickets, assignment, status, internal notes, customer replies, knowledge base links. | Link tickets to contacts; add email intake through an existing provider. |
| 7 | Time tracking | Toggl Track, Harvest | Timers, manual entries, billable hours, project budgets, reports. | Attach time to projects; later supply invoice line items. |
| 8 | Appointment scheduling | Calendly, Acuity | Availability, booking pages, basic intake, buffers, cancellation, rescheduling, reminders. | Forms adds custom intake; CRM adds sales follow-up. Calendar synchronization is a distinct integration milestone. |

The initial suite supports publishing content, capturing interest, managing a sale, delivering work, recording time, providing support, and booking appointments. Build order and customer journey order are different: scheduling is eighth to keep its calendar and concurrency work out of the first release.

Each application must pass its standalone journey with optional applications disabled. An optional connection may require both applications, but the base application must remain useful without it.

## Ordered expansion backlog

| Order | Application | Reference products | Bounded scope and sequencing rationale |
| --- | --- | --- | --- |
| 9 | Customer feedback and roadmaps | Canny, Productboard feedback workflows | Requests, votes, prioritization, roadmap, changelog; connects customers, tickets, and projects. |
| 10 | Client portals | SuiteDash, Softr client portal workflows | External access to selected projects, files, requests, and status; builds on useful underlying records and scoped access. |
| 11 | Quotes and invoicing | FreshBooks invoicing workflows | Quotes, invoice documents, numbering, due dates, reminders, manually recorded payments; payment processing remains external. |
| 12 | Expense tracking and approvals | Expensify expense workflows | Receipts, categories, submissions, approvals, export; excludes reimbursement rails and tax filing. |
| 13 | Equipment and asset tracking | Sortly asset workflows | Assets, assignments, checkout, maintenance dates, labels. |
| 14 | Inventory and purchasing | inFlow, Zoho Inventory | SKUs, locations, stock movements, suppliers, purchase orders; requires transactional stock invariants. |
| 15 | Employee directory, leave, and onboarding | BambooHR core HR workflows | Restricted employee records, leave requests, onboarding checklists; excludes payroll and benefits administration. |
| 16 | Applicant tracking | Workable ATS workflows, Breezy HR | Job pages, applications, candidate stages, interview notes; excludes sourcing networks. |
| 17 | Training and learning management | TalentLMS, Trainual | Courses, lessons, quizzes, assignments, completion tracking. |
| 18 | Digital asset management | Bynder library workflows, Brandfolder | Searchable media, tags, versions, collections, controlled sharing; extends shared file capabilities. |
| 19 | Landing page and simple website builder | Unbounce, Leadpages | Templates, sections, CMS content, forms, publishing; visual editing is a substantial separate scope. |
| 20 | Email campaign management | Mailchimp campaign workflows | Lists, segments, templates, campaigns, unsubscribe handling; delivery uses an external provider. |
| 21 | Website and product analytics | Mixpanel, Amplitude core analytics | Event collection, dashboards, funnels, retention; validate storage growth and query performance separately. |
| 22 | Reporting and BI | Databox, dashboard-focused Power BI workflows | Saved queries, charts, filters, scheduled reports; benefits from established operational data. |
| 23 | Workflow automation builder | Zapier, Make | User-configured triggers, conditions, actions, schedules, run history; extends the reliable recipe execution already used by earlier applications. |
| 24 | Custom databases and spreadsheet-style applications | Airtable, Smartsheet | Typed tables, relationships, views, formulas, permissions; avoid making this a prerequisite for the concrete applications. |
| 25 | Internal tool builder | Retool | Connections, tables, forms, actions, permissions; generalize only after real applications establish useful reusable patterns. |

Expansion order is a planning preference, not a dependency chain. A customer need can justify moving an application earlier without requiring all preceding applications to be enabled or built.

## Replacement standard

Every release states the specific workflow it replaces, the source formats it can import, what users can export, and what remains unsupported. A migration claim requires a tested importer for the named source and scope; generic CSV support alone is not proof of full vendor migration.

Installation, authorization, reliable background work, update compatibility, and recovery are part of the replacement. An attractive demonstration is insufficient evidence that a business can cancel its subscription.

## Reference anchors

These references informed the initial workflow comparisons; they are not frozen vendor specifications or performance evidence.

- [Pipedrive CRM features](https://www.pipedrive.com/en/crm/features)
- [Typeform platform overview](https://www.typeform.com/platform-overview)
- [Contentful content models](https://www.contentful.com/help/content-models/)
- [Airtable automation concepts](https://support.airtable.com/articles/3669392397-Getting-started-with-airtable-automations)

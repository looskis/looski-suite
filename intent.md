# Business software suite

## Purpose

Build open-source replacements for software that small and medium-sized businesses currently pay to use. The applications should run on hardware the business controls, be easy to install on Apple Silicon, and compose into a useful suite without requiring every application to be installed.

This is the canonical entry point for the suite plan as of October 4, 2026. It defines product intent and scope; the linked documents own architecture, decisions, terminology, and build milestones. These are design commitments for a new suite, not claims about an implemented or deployed Looski application.

## Thesis and challenge

### Original thesis

> SaaS is dead. Over the next *n* videos, I will prove this to you by rebuilding a major SaaS product from scratch within a single day.
>
> Every single replacement will be open source and designed to run directly on your own machine. All it takes is handing the repo to your local agent to configure and deploy in under five minutes.

The selected license is AGPL-3.0 with a separate trademark policy. Commercial use, resale, and paid hosting are allowed subject to the license's copyleft and corresponding-source requirements. See the [licensing decision](docs/adr/0006-agpl-and-trademark-policy.md).

The target is **pure SaaS**: products whose primary deliverable is software, often an opinionated interface over a database, business workflows, and CRUD operations. Initial examples include CRMs, content management systems, project management, internal tools, and form builders.

The product proposition is ownership, useful functionality, simple operation, and freedom to select the applications a business needs.

### How to make the challenge credible

Reproducing a useful workflow in a day is different from replacing every capability and operational responsibility of an established vendor. Each release should state the workflows it supports, its limitations, and the evidence that it is usable.

An earlier episode promise was proposed, but has not replaced the original hook:

> Each episode, I’ll rebuild the core workflows a small business uses in one paid SaaS product. The replacement will be open source, run on hardware you control, and install in under five minutes. I’ll show what works, what’s missing, and whether it’s ready to replace the subscription.

The shared installation, authentication, permissions, backup, and module foundation can be reused across episodes. That reuse should be disclosed when describing a one-day build. The five-minute installation target remains a goal to validate on a clean machine, not an achieved result.

### Excluded categories

The original challenge excludes the following categories. These are boundaries for the project, not assertions that every listed company has the same moat or that its software has no independent value.

| Category | Examples from the original thesis | Why it is outside the challenge |
| --- | --- | --- |
| Frontier AI labs and foundation models | OpenAI, Anthropic | Frontier compute, research, training infrastructure, and data pipelines exceed the scope of reproducing business application workflows. |
| Regulated financial services and payment rails | Stripe, Robinhood, Ramp, Alpaca | Production financial services involve external networks, regulatory obligations, risk, and operational responsibilities that an application clone does not reproduce. |
| Telephony, carrier, and physical delivery services | Twilio, SendGrid, Lob | Deliverability, carrier relationships, and physical delivery are external services beyond the local software. |
| Selected collaboration, counterparty, and legal products | DocuSign, Carta, Figma | These products remain outside the challenge; their value can include collaboration, interoperability, trust, and substantial specialized software. Ordinary team collaboration inside the planned applications remains in scope. |
| Hyper-scale infrastructure and managed compute | AWS, Cloudflare, Snowflake, Datadog | A local implementation does not reproduce global infrastructure, distributed operations, or managed service commitments. |
| Proprietary data and intelligence | ZoomInfo, Bloomberg Terminal, PitchBook, LexisNexis | Recreating a UI does not reproduce proprietary datasets, exclusive feeds, or ongoing data maintenance. |
| Identity verification and compliance platforms | Persona, Clear, ID.me, Vanta, Drata | Identity verification, compliance automation, evidence workflows, and independent assurance introduce different obligations beyond this initial scope. |

Two distinctions raised in discussion should be retained when refining the public argument:

- Figma is not primarily a legal-verification platform. Its editor and multiplayer capabilities are substantial software products.
- Vanta and Drata automate compliance work; they should not be described as interchangeable with identity-verification providers or independent auditors. A platform's assistance does not itself constitute an audit.

## Canonical documentation

| Document | Owns |
| --- | --- |
| [Architecture](docs/architecture.md) | Deployment, identity, authorization, module selection, shared Contacts, data ownership, workflows, operations and reliability. |
| [Application roadmap](docs/roadmap.md) | Ordered replacement scopes: eight initial applications and seventeen expansion candidates. |
| [Setup contract](docs/setup-contract.md) | Migration workflow, installation identity, proposed ports/dependencies, and Cloudflare routing. |
| [Build plan](docs/build-plan.md) | Implementation milestones and measurable acceptance gates. |
| [CRM planning record](docs/crm-plan.md) | Evolving CRM requirements, proposed contracts, unresolved product choices, and planning history. |
| [Decisions](docs/decisions.md) | Accepted direction, open choices, and links to decision rationale. |
| [Local backup and recovery](docs/local-backup-and-recovery.md) | Proposed local backup contents, recovery procedure, and release evidence. |
| [Business glossary](CONTEXT.md) | Canonical terms, including user versus contact and submission versus lead. |

The owning document is authoritative for its subject. Keep summaries and references consistent when changing a decision; record significant trade-offs in [architecture decision records](docs/adr/0001-modular-monolith-and-module-selection.md). Open choices remain open until evidence and a decision are recorded.

## Accepted product direction

Build one open-source suite whose applications work independently and together. Businesses choose which applications to enable in each workspace. Members see and use only what their permissions allow within those enabled applications.

A small shared foundation supplies identity, workspaces, membership, permissions, Contacts, files, jobs, audit history, installation, updates, and recovery. Contacts represents business people and organizations independently of CRM and login accounts. Enabling an application makes compatible built-in connections available for explicit configuration; it does not automatically activate outbound workflows.

Expose supported business capabilities through a CLI, an HTTP API, and MCP alongside the web UI. All interfaces use the same Rust business operations, workspace permissions, and module rules. Public third-party OAuth remains a future capability.

Use one standard distribution initially, with application selection at runtime. Disabling an application blocks its access and workflows while preserving data; deletion is separate. Packages that physically omit unused code and third-party runtime plugins are deferred.

The business backend is Rust, business data lives in PostgreSQL, operational logs go to local ClickHouse, authentication is Supabase Auth, and the frontend is Svelte with SvelteKit, TypeScript, and SSR. Axum, SQLx, `utoipa`/`utoipa-axum`, SvelteKit, and PGMQ are accepted. Support email/password and magic-link login, with Cloudflare Tunnel for public HTTPS access to configured local service ports. Remaining integration choices are recorded in the architecture and decision register.

Supabase Auth is a required foundation dependency in both hosting modes. Self-hosted Supabase uses pinned upstream Docker images and its standard Auth schema; Docker and Compose are accepted deployment dependencies for the local distribution. We accept the container runtime overhead and will measure it rather than build a native distribution to avoid an unmeasured cost. Hosted Supabase is a separate instance selected through the same application integration, with no automatic synchronization between local and hosted data or identities. Moving existing data is an explicit migration and cutover.

## Initial application order

1. CRM.
2. Forms and surveys.
3. CMS.
4. Projects and tasks.
5. Knowledge base and SOPs.
6. Helpdesk.
7. Time tracking.
8. Appointment scheduling.

These eight are the initial roadmap. The remaining seventeen applications are an ordered expansion backlog, not a commitment to implement every candidate before learning from customers. See the [full roadmap](docs/roadmap.md) for replacement scope, reference products, and optional connections.

## Flagship connected journey

A customer books an appointment and completes an intake form. The suite saves the appointment and answers, finds or creates the appropriate contact, records the configured CRM inquiry, and shows the interaction on an authorized contact timeline. A CRM failure does not undo a confirmed appointment; follow-up work is durable and recoverable.

Scheduling also works without Forms or CRM. Forms adds custom intake; CRM adds sales follow-up. This combination of independent usefulness and explicit connections is the model for the other applications.

## First build and evidence

Start with the smallest complete CRM journey: install, bootstrap an administrator, invite a restricted teammate, import and manage business records, verify denied access, select applications, restart, export, back up, and restore. Build the shared foundation through that journey.

Next connect Forms to CRM, then continue in roadmap order. Keep a precise replacement claim for every release, test source-specific migration before claiming it, and disclose reused foundation work and external services. One-day builds and five-minute installation remain goals until the [build plan's acceptance gates](docs/build-plan.md) provide evidence.

Current status: documentation and design only. No implementation, installation benchmark, or production-readiness result is established by this plan.

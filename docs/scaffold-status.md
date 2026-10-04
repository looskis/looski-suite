# Initial scaffold implementation

Historical record; superseded for identity/session behavior by the [October 12 identity slice](identity-slice.md).

Date: October 4, 2026. Scope: build-plan step 1 plus a local dependency integration probe. This is not completion of phase 0 or the first CRM release.

## Implemented

- Rust workspace with Axum 0.8.9, SQLx 0.9.0, utoipa 6.0.0, and utoipa-axum 0.3.0. One executable supports `api`, `openapi`, `status`, and `mcp` modes.
- Public system metadata goes through one application operation. CLI and stdio MCP use the HTTP API, never direct database access. MCP uses the official Rust SDK (`rmcp` 3.5.0). No business operations or permission grants exist yet.
- SvelteKit 2.62.0/Svelte 5.57.1 SSR, TypeScript 5.9.3, Node adapter 5.5.4. This compatible version set keeps the scaffold on Kit 2; a major-version upgrade can be tested separately.
- `openapi-typescript` 7.13.0 plus `openapi-fetch` 0.17.0 for a typed SSR request. Generated OpenAPI and TypeScript contracts are checked for drift without modifying the checked-in files.
- Compose project `looski-suite`, loopback gateway `58888`, private service ports, scoped volumes/secrets, and bounded container logs. Caddy 2.10.2 supplies both routing layers for the small Auth-only subset.
- Official Supabase PostgreSQL `17.6.1.136` and Auth `v2.196.0`, with bootstrap SQL sourced from `self-hosted/v0.8.2`. The required webhooks bootstrap establishes an upstream role used by the supplied roles script; packaging Auth alone must still preserve this bootstrap dependency. Suite-owned initialization does not replace Auth migrations.
- Database bootstrap completion sentinel plus health checks. An init failure followed by a Postgres restart cannot be mistaken for completed bootstrap. Password-setting SQL suppresses statement logging.
- Supabase CLI 2.75.0 in a serialized migration job, explicit local DB target, and a fresh migration container for every `make up`. The first suite migration enables PGMQ and a private schema-version table. Runtime role is separate from the migration role and has no schema administration grants.
- Optional private ClickHouse packaging; collection and retention are not implemented. No API readiness dependency on ClickHouse. Optional Cloudflare tunnel connector; tunnel/DNS provisioning is an explicit operator step.
- Standard AGPLv3 license text, upstream Apache license/attribution, Cargo/pnpm lockfiles, formatting/lint/type-check/test commands, and a GitHub Actions workflow.

## Validation record

- Host: Apple Silicon (`aarch64` Docker engine 29.6.1), Docker Compose 5.3.0, Rust 1.97.1, Node 26.3.1, pnpm 11.24.0, Python 3.9.
- Passed Rust formatting and Clippy with warnings denied; two Rust tests cover dependency outage versus liveness and closed unimplemented business routes.
- Passed Svelte type checking without warnings and a production SSR build.
- Passed OpenAPI/TypeScript drift checks and four setup configuration/secret guard tests.
- Built the Rust, web, and Auth wrapper images for ARM64.
- Verified the Supabase image bundles PGMQ 1.5.1 and that upstream Auth initializes with the retained bootstrap dependencies.
- Verified ClickHouse 25.8.4.13 starts healthy on ARM64 with private ports. Stopped it and confirmed API readiness remains healthy. Ingestion is still pending.
- Passed complete stack smoke on the default installation and a fresh, separate `looski-suite-scaffold-check` installation at port 58889: SSR HTML, public API/OpenAPI, CLI, MCP initialize/list/call, readiness, closed routes, upstream Auth migrations, schema grants, and real PGMQ/write rollback. Both installations ran concurrently using separate volumes and secrets. The secondary test installation is now stopped; its empty development volumes and secrets are retained. The primary preview remains at http://localhost:58888.
- Passed setup rerun: secret bytes were unchanged, the migration container ID changed, and end-to-end smoke passed again. The final start sequence does not restart the completed migration job twice.
- Code review found and corrected inherited OpenAPI library metadata and Linux file-backed secret UID access. Rebuilt smoke passed, API process UID/GID is 10001 and Auth is 1000; the API connection file is mode 0400 and owned by 10001 in tmpfs. No outstanding review findings remain.

## Boundaries and next slice

Only installation metadata and health are public. Auth token/admin endpoints and business paths are closed. This does not implement identity validation, sessions, invitations, magic links, permissions, or third-party OAuth. Those must precede Contacts operations.

Next: verified Supabase identity, bootstrap/login, workspace membership and authorization denial tests; then one Contacts create/list operation with a module-owned SQLx DAO, audit and queue transaction, exposed through all three interfaces and the web UI. No speculative Contacts/opportunity/deal schema or role presets were added.

The [durable jobs checkpoint](implementation/status-jobs-slice.md) now implements worker execution, retries, fenced crash recovery and explicit module resume.

The [starter package checkpoint](implementation/starter-package-slice.md) adds explicit editable defaults and safe retirement; upgrades never reinstall them.

Still pending: migration-content checksum drift rejection, an upgrade from a previous application release, backup/restore, hosted Supabase overlay, callback/SMTP/Cloudflare testing, collector ingestion/redaction/retention, resource measurements, vulnerability/dependency license checks, corresponding-source delivery, and trademark policy. The GitHub Actions workflow is authored but has not run on GitHub.

The initial database probe uncovered two integration errors (a missing upstream role dependency and execution of a bind-mounted shell sentinel); both were corrected in the scaffold. The failed probe volumes contained no user/business data. A test runtime credential emitted by upstream statement logging was replaced, and password-setting statements now suppress that logging. Setup never automatically resets volumes or rotates existing secrets.

The pinned migration CLI also requires an explicit `PGSSLMODE` environment setting for the private local database; a URL parameter alone did not work in the probe. The upstream CLI has a [reported URL SSL-mode handling issue](https://github.com/supabase/cli/issues/5873). Hosted mode must use verified TLS and is deliberately not enabled by this scaffold.

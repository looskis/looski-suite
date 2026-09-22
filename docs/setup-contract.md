# Installation and setup contract

Status: accepted migration location/runner, `looski-suite` installation namespace, and default local gateway port `58888`; internal port defaults, routing layout, and setup workflow remain proposed, October 4, 2026. The initial development Compose stack and setup wrapper are implemented; see [scaffold evidence](scaffold-status.md) for validated behavior and remaining gates. This document owns setup configuration and endpoint planning; the architecture owns application behavior and security requirements.

## Migration ownership

All suite-owned PostgreSQL schema changes live in `supabase/migrations/<timestamp>_<description>.sql`. Use the Supabase CLI to create and apply those migrations, with one migration history. SQLx remains the application query/transaction library; do not add an independent SQLx migration directory, runner, or history. Module ownership still applies even though migration files share one chronological directory.

Use a pinned CLI version and an explicit installation target. `supabase migration new <name>` creates migration files; `supabase migration up --db-url <target>` applies them to a specified self-hosted database. The setup wrapper should resolve the connection securely, identify the target without printing credentials, and invoke the supported CLI path. Hosted mode uses the same files against the explicitly selected hosted target. It must not choose a linked hosted project merely because a developer previously linked one. Validate the exact runner packaging and secret handling before implementing it.

The database URL inside the Compose network points to `db:5432`; it is not the browser gateway URL. A migration job on that network avoids publishing PostgreSQL just for migrations. Do not run `supabase start` as the deployment launcher for an additional stack alongside Compose. The CLI's local stack settings in `supabase/config.toml` do not automatically configure a separately managed Compose deployment.

Supabase-managed bootstrap, roles, `auth` schema, and Auth upgrades remain managed by the pinned upstream services/scripts. Suite migrations operate outside `auth`; prerequisites such as PGMQ availability and Supabase-managed initialization must be verified before the suite migrator runs. ClickHouse has its own schema/setup mechanism; PostgreSQL migrations cannot migrate it.

Use one serialized migration job before releasing a new application version. Each update must execute a fresh job against the target schema; do not treat a previous release's successful container exit as proof that new migrations ran. Verify clean-database replay and upgrade from the previous supported version, reject migration-history drift, and preserve applied migrations by adding new ones instead of editing history. Runtime database credentials cannot perform schema administration. Development seeds must not run automatically against a business installation. Reset/down operations are separate destructive development/recovery actions, never ordinary setup or update steps.

References: [Supabase migration layout](https://supabase.com/docs/guides/deployment/database-migrations) and [explicit-target migration command](https://supabase.com/docs/reference/cli/supabase-migration-up).

## Installation identity and Docker names

Use `looski-suite` as the default Compose project name. Examples of generated resources are `looski-suite-db-1`, `looski-suite-auth-1`, and project-scoped named volumes/networks. Keep upstream image references and pinned versions/digests intact; image names identify reusable software, not a running installation.

Remove upstream fixed `container_name` values when assembling the distribution, and update any references to use service DNS names. A project name does not namespace explicitly fixed container names, external resources, or shared host bind-mount paths. Avoid globally named volumes/networks; put any required writable host paths under an installation-specific directory.

For two simultaneous installations, choose identities such as `looski-suite-dev` and `looski-suite-demo`, distinct host bindings, and separate data/secrets/tunnel configuration. A second invocation with the same project identity addresses the same installation. Persist the chosen name; changing it casually can make the current data volumes appear absent. A future CLI-managed development stack may use a matching unique Supabase `project_id`, but that setting is not the Compose namespace.

Proposed Compose naming fragment:

```yaml
name: ${COMPOSE_PROJECT_NAME:-looski-suite}
```

See [Compose project naming](https://docs.docker.com/compose/how-tos/project-name/). Installer commands must use the same resolved project name, working directory, Compose files, and configuration on every run.

## Canonical ports and endpoints

Use fixed internal service ports and a configurable host gateway binding. Services talk to Compose DNS names, never another container's `localhost`. The default host gateway port `58888` is selected; internal service ports remain proposed. These defaults do not reserve host ports or claim those ports are currently free.

| Service | Proposed internal address | Default host publication |
| --- | --- | --- |
| Suite gateway | `gateway:8080` | `127.0.0.1:58888` → container `8080` |
| SvelteKit | `web:3000` | None |
| Rust API | `api:8080` | None |
| Supabase gateway, where required by validated upstream configuration | `supabase-gateway:8000` | None |
| Supabase Auth | `auth:9999` | None |
| PostgreSQL and PGMQ | `db:5432` | None |
| ClickHouse HTTP/native | `clickhouse:8123` / `clickhouse:9000` | None |
| Proposed OpenTelemetry collector | `collector:4317` / `collector:4318` | None |
| Worker, migrator, cloudflared | No application listener required | None |

Supabase helper services retained after validation stay internal as well. Different containers can listen on the same internal port. Only host bindings collide across installations. Optional debugging/admin bindings must be explicit loopback-only overrides with collision checks; the standard installation need not expose database, Studio, ClickHouse, or raw Auth ports.

The gateway publishes on both `127.0.0.1` and `::1`, with no wildcard LAN binding. This is required for reliable `localhost` clients on the supported Mac runtime: an unbound IPv6 destination in the ephemeral port range can accidentally TCP-connect back to the client itself. Setup checks both families; the smoke suite verifies actual HTTP responses and reserved listener endpoints.

A bind conflict should fail with the conflicting setting and a suggested override; do not silently change a port because callbacks and configured clients depend on it. A preflight check is helpful but cannot replace handling a bind failure at startup. For a second installation, change both project identity and host gateway port, for example to `58889`.

## Configuration sources

The scaffold now implements these core configuration sources. Cloudflare provisioning and hosted-mode overlays remain pending.

| File or mechanism | Owns |
| --- | --- |
| `compose.yaml` | Service definitions, pinned image references, internal ports/networks, volumes, health checks, and dependency conditions. |
| `.env.example` | Documented non-secret installation defaults and required variable names. |
| Installation configuration (`.env` or equivalent private file) | Project identity, host port, deployment mode, and public application origin. |
| Private secret files / supported secret injection | Database credentials, signing configuration, SMTP credentials, and tunnel credentials. Never committed or included in diagnostic output. |
| `supabase/config.toml` | Supabase CLI project metadata and any deliberately supported CLI workflows; not the runtime settings authority for Compose. |
| `supabase/migrations/` | Suite PostgreSQL schema history. |
| Gateway routing template and rendered config | Routing derived from service definitions and installation settings. |
| cloudflared configuration | The selected hostname-to-gateway route and tunnel identity, using the same installation inputs. |

Keep the user-facing configuration small: proposed `COMPOSE_PROJECT_NAME`, `LOOSKI_HTTP_PORT`, `LOOSKI_DEPLOYMENT_MODE`, and `LOOSKI_PUBLIC_URL`. For example, local mode can resolve the public origin to `http://localhost:58888`, while tunnel mode uses `https://suite.example.com`. Use one hostname consistently in browsers and callbacks rather than interchanging localhost and IP addresses.

Generate derived Auth URLs, redirect allowlists, SvelteKit origin settings, gateway routes, and tunnel routes from those values. Do not ask operators to maintain several copies of the same port or origin. Changing the public origin requires a coordinated configuration update and may require fresh login; it is not merely renaming a DNS record. Hosted mode supplies the chosen external Supabase endpoint/database and uses a tested Compose overlay without local database/Auth startup dependencies.

## Dependencies and readiness

Declare runtime process dependencies in Compose using health checks and conditions, not only startup order. Use `service_healthy` for ready services and `service_completed_successfully` for the one-shot migration job. The complete Supabase dependency graph must follow the validated pinned upstream distribution, not a guessed two-container subset.

Proposed local sequence:

1. Start the database and required upstream initialization/services; verify readiness including Auth prerequisites and required extensions.
2. Run the suite migration job once against the selected database. Failure blocks the application release and worker startup.
3. Start the API and worker only against a compatible schema; verify Auth connectivity/keys before declaring authentication ready.
4. Start the web runtime and gateway, then verify the complete local login/API path.
5. If public access is configured, start cloudflared and verify the intended hostname and callback path.

ClickHouse and the collector start on an independent observability path. They must not be hard startup/readiness dependencies of CRM writes. Runtime connection retries and error handling are still required: Compose startup conditions do not heal every later outage. A worker must not start processing while an incompatible migration is being applied.

Software dependencies belong in the Cargo and JavaScript manifests and committed lockfiles. Installation prerequisites belong in the setup checks: supported Docker/Compose versions and runtime, architecture, disk space, and compatible release images. Business-module dependencies remain in the application registry; enabling CRM does not launch a separate Compose deployment.

See [Compose dependency readiness](https://docs.docker.com/compose/how-tos/startup-order/).

## Cloudflare Tunnel routing

Proposed default: run cloudflared in the suite network and route one public hostname to `http://gateway:8080`. That route uses an internal service address, not the Mac's published port. If an existing host-run connector is used instead, route it to `http://127.0.0.1:58888` (or the configured override). Do not use `localhost` inside a connector container to refer to another container or the host.

Illustrative locally managed tunnel ingress, not a complete deployed configuration:

```yaml
ingress:
  - hostname: suite.example.com
    service: http://gateway:8080
  - service: http_status:404
```

Provisioning the actual tunnel identity, credentials, and DNS hostname route is also required. An ingress entry alone does not create DNS or a tunnel. An API-managed tunnel may express the same route in its remote configuration; choose one configuration authority, not both.

The suite gateway routes `/api/v1/…` to Rust, the approved public `/auth/v1/…` endpoints through the validated Supabase gateway/Auth path, and frontend routes (including `/auth/confirm`) to SvelteKit. Apply the required prefix preservation/rewrite rules explicitly. Keep Auth administrative endpoints and the Supabase Data API, Studio, PostgreSQL, and ClickHouse off the public route. Validate allowlisted routes and denial behavior rather than exposing the entire upstream Supabase gateway.

Set the Supabase external API URL and issuer expectations, Auth Site URL and allowed redirects, and SvelteKit public origin consistently. Preserve trusted proxy headers correctly; do not trust arbitrary client-supplied forwarded headers. Verify magic links, secure cookies, CSRF/origin checks, and that authenticated responses are not cached publicly. Cloudflare provides HTTPS reachability; Rust still authenticates and authorizes API requests.

See [Cloudflare ingress configuration](https://developers.cloudflare.com/tunnel/features/locally-managed-tunnels/configuration-file/). Gateway implementation and local/API-managed tunnel provisioning mode remain choices to validate in phase 0.

## Proposed setup experience

The development `make up` command implements the initial configuration/health/migration sequence. First-admin setup, manual encrypted backup/recovery, and migration checksum guards are implemented; hosted mode and public-provider validation remain release gates. See [recovery](recovery.md) and [diagnostics](diagnostics.md). The eventual supported installer must resolve one installation identity/configuration, check prerequisites and collisions, create secrets only for a new installation, render deployment configuration, start dependencies, apply pending migrations, verify health, and guide first-admin authentication. It must be safe to rerun without resetting storage, rotating identity keys unexpectedly, or reopening bootstrap access. Tunnel/domain and SMTP configuration should be explicit steps, with useful local setup possible before public routing is configured.

Status/diagnostics should show the installation identity, release/schema versions, local/public URLs, service health, pending migrations, and configuration problems without printing secrets. Optional cleanup must target only this installation; removing persistent data is distinct from stopping services.

Phase 0 evidence: two installations coexist with distinct names/ports/volumes; migration replay/upgrade targets the intended database; failed migrations block incompatible app startup; a port collision is actionable; tunnel routing and Auth callbacks work; administrative/data ports remain private; and a ClickHouse outage does not block business operations.

## Implemented operational settings

`LOOSKI_MAIL_MODE` selects disabled, local capture, or configured SMTP. `LOOSKI_DIAGNOSTICS_MODE` selects local ClickHouse (default) or disabled. Setup derives the associated profiles and private credentials; diagnostic startup failure does not block business services. The first diagnostic pipeline is a bounded in-process exporter, so no OpenTelemetry collector service is required.

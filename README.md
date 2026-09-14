# Looski Suite

Business software that works independently and together. Rust/Axum owns business operations; SvelteKit renders the web application. PostgreSQL, Supabase Auth, and PGMQ form the local foundation.

**Development scaffold:** the web shell, public system API, CLI, local stdio MCP, and dependency health checks are implemented. Login, workspace permissions, Contacts/CRM, background job execution, and production operations are the next slices. Do not use this scaffold for business data or expose it as a production service.

## Start locally

Prerequisites: a macOS or Linux host, Docker with Compose v2.20+ (a running Linux engine), Python 3.9+, and enough disk space for the Rust/Node and Supabase images. The complete stack builds inside Docker; host Rust/Node are needed only for development and checks.

```sh
make up
```

Open [localhost:58888](http://localhost:58888). The first run builds images and initializes a new development database. Subsequent runs preserve secrets/data, stop application callers, run a fresh migration job, and restart healthy services. `make stop` stops containers and preserves volumes. `make status` shows service state.

`make setup` creates the non-secret `.env` configuration and private `.local/looski-suite/secrets/` files without starting services. Keep those secrets with this installation; missing/partial secrets are an error, never a reason to silently rotate passwords. The wrapper has no reset/delete-data command.

The default configuration is:

```dotenv
COMPOSE_PROJECT_NAME=looski-suite
LOOSKI_HTTP_PORT=58888
LOOSKI_PUBLIC_URL=http://localhost:58888
```

Only the gateway binds a host port, on loopback. Web, API, Auth, PostgreSQL, and the Supabase gateway communicate using Compose DNS. For a separate installation, consistently supply a different project identity and port/origin on every command; secrets and named volumes are scoped to that identity:

```sh
COMPOSE_PROJECT_NAME=looski-suite-demo LOOSKI_HTTP_PORT=58889 \
  LOOSKI_PUBLIC_URL=http://localhost:58889 make up
```

The root `.env` remains the default installation; command-specific overrides do not rewrite it. Do not casually rename an existing installation: its data and secrets belong to the original project name.

## Develop and verify

Host tools: Rust 1.97.1, Node 22.17+ (validated with 26.3.1), pnpm 11.24.0, and Python 3.9+. Cargo and pnpm lockfiles are committed. The container builds pin their own toolchains.

```sh
pnpm install --frozen-lockfile
make check test
pnpm build
cargo build --locked
make smoke                 # requires make up first
```

`make check` runs Rust formatting/Clippy, frontend formatting/type checks, and generated-contract drift checks. `make test` runs dependency-outage/closed-route tests and setup guards. `make smoke` checks the running SSR page, API, OpenAPI, CLI, MCP, Auth schema, role isolation, and PostgreSQL/PGMQ atomic rollback.

For frontend iteration, run `LOOSKI_API_INTERNAL_URL=http://localhost:58888 pnpm dev` and open `http://localhost:58080`. That port is an explicit host development server; the packaged web service stays private on container port 3000. Native API iteration requires an explicit private database connection file and Auth URL; no database debug port is published automatically.

## API, CLI, and MCP

Only public installation metadata is available; no endpoint accepts business mutations or implements login yet.

```sh
curl http://localhost:58888/api/v1/system
cargo run --locked -p looski -- status
cargo run --locked -p looski -- mcp
cargo run --locked -p looski -- openapi
```

The MCP command speaks JSON-RPC on stdin/stdout with diagnostics on stderr. It advertises one tool, `system_status`, which calls the same HTTP API as the CLI. A local MCP client can launch `target/debug/looski mcp` with `LOOSKI_API_URL=http://localhost:58888`. Credential storage, authenticated tools, and external OAuth are not implemented.

The generated HTTP contract is served at `/api/v1/openapi.json` and checked in at `contracts/openapi.json`. Run `pnpm api:generate` after changing endpoint metadata. `openapi-typescript` generates frontend types; `openapi-fetch` provides typed calls. Public metadata has no bearer security requirement; business security metadata must be added with verified identity and permissions.

## Layout

| Path | Responsibility |
| --- | --- |
| `crates/looski/src/application.rs` | First transport-independent operation; grow module-owned business operations and focused SQLx DAOs here as real journeys arrive. |
| `crates/looski/src/api.rs` | HTTP routes, DTOs, health, and OpenAPI. |
| `crates/looski/src/client.rs`, `mcp.rs` | CLI/API client and local MCP adapter. |
| `apps/web/` | SvelteKit SSR UI and generated API types. |
| `supabase/migrations/` | Sole suite PostgreSQL migration history, applied by the pinned Supabase CLI. |
| `infra/`, `compose.yaml` | Pinned runtime images, upstream bootstrap, routing, migration job, and optional ClickHouse. |
| `scripts/` | Development setup, contract generation, and verification. |
| `docs/` | Canonical architecture, decisions, product/build plans, and implementation evidence. |

Avoid empty repository/service abstraction layers. Introduce each module's DAO and authorization interface with the first operation that needs it. Transport handlers must not become owners of business rules.

## Infrastructure boundaries

The Auth-only subset uses upstream Supabase PostgreSQL/Auth images and bootstrap SQL from `self-hosted/v0.8.2`. Caddy provides the suite gateway and the small internal Auth gateway. We do not package Studio, PostgREST, Storage, Realtime, or Edge Functions. `/auth/v1/health` is the only externally routed Auth endpoint in this slice; administrative, token, and data routes remain closed until the authentication slice defines their access contract.

API and Auth entrypoints read private file-backed secrets before dropping to their non-root service users. The API copies its connection file into tmpfs with mode `0400`; secrets do not enter image layers or Compose environment output.

Postgres bootstrap creates a non-administrative `looski_runtime` login. The suite migration grants it only schema usage and schema-version reads for now. The runtime cannot migrate the suite schema. Auth owns its own migrations; SQLx never runs schema migrations.

Optional ClickHouse packaging can be started with `docker compose --profile observability up -d clickhouse`. It has private ports, persistent data, and loopback-only database access inside its container. Collector ingestion, credentials, retention, and async outage behavior remain a separate probe. Application diagnostics currently use bounded Docker logs; no ClickHouse ingestion is claimed.

Cloudflare provisioning is not automated. A host-run tunnel can target `http://127.0.0.1:58888`; a connector on the Compose network targets `http://gateway:8080`. See the [example ingress](infra/cloudflare/config.example.yml) and [setup contract](docs/setup-contract.md). Public-domain callback/login validation waits for the authentication slice. Hosted Supabase mode, worker execution, migration checksum drift enforcement, backup/restore, and release/source-distribution packaging are not complete.

## Canonical plans and license

Start with the [build plan](docs/build-plan.md), [architecture](docs/architecture.md), [decisions](docs/decisions.md), and [scaffold evidence](docs/scaffold-status.md). Original software is licensed under [AGPL-3.0-only](LICENSE); upstream components retain their own licenses. See [NOTICE](NOTICE). The separate trademark policy and release compliance remain release gates.

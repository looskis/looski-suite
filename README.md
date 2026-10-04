# Looski Suite

Business software that works independently and together. Rust/Axum owns business operations; SvelteKit renders the web application. PostgreSQL, Supabase Auth, and PGMQ form the local foundation.

**Local CRM build:** Contacts, organizations, reviewed intake, opportunities, separate deals, tags, shared Tasks/Files, activities, coordination templates, staff invitations, scoped role administration and business export work through web, API, CLI and local stdio MCP. The build also includes local authentication mail, durable jobs, encrypted recovery and optional ClickHouse diagnostics. See [coverage and release boundaries](docs/implementation/coverage.md); external SMTP/public callbacks, hosted migration and public distribution still require their release checks.

## Start locally

Prerequisites: a macOS or Linux host, Docker with Compose v2.20+ (a running Linux engine), Python 3.9+, and enough disk space for the Rust/Node and Supabase images. The complete stack builds inside Docker; host Rust/Node are needed only for development and checks.

```sh
make up
```

Open [localhost:58888](http://localhost:58888). The first run builds images and initializes a new development database. Before updating an existing installation, create and verify an [encrypted recovery snapshot](docs/recovery.md). Subsequent runs preserve secrets/data, stop application callers, run a fresh migration job, and restart healthy services. `make stop` stops containers and preserves volumes. `make status` shows service state.

Create your first owner after startup:

```sh
python3 scripts/bootstrap_owner.py
```

This local operator command prompts for your email, name, workspace, and password without echoing it. It confirms the initial owner’s email locally, creates the Supabase account, and atomically records the workspace, membership, permission, audit event, and permanent bootstrap latch. It cannot run again after success. If interrupted, retry with the same credentials; it never changes an existing account’s password. Open [sign-in](http://localhost:58888/login). Magic links, password recovery and invitation delivery support private development mail or configured SMTP; see [authentication email](docs/auth-email.md). Email matching alone never grants workspace access.

`make setup` creates the non-secret `.env` configuration and private `.local/looski-suite/secrets/` files without starting services. Keep those secrets with this installation; missing/partial secrets are an error, never a reason to silently rotate passwords. The wrapper has no reset/delete-data command.

The default configuration is:

```dotenv
COMPOSE_PROJECT_NAME=looski-suite
LOOSKI_HTTP_PORT=58888
LOOSKI_PUBLIC_URL=http://localhost:58888
LOOSKI_MAIL_MODE=disabled
LOOSKI_DIAGNOSTICS_MODE=local
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

`make check` runs Rust formatting/Clippy, frontend formatting/type checks, and generated-contract drift checks. `make test` runs unit/contract, bounded-diagnostics, setup, migration-integrity and archive/recovery guards. `make smoke` checks the running SSR page, API, OpenAPI, CLI, MCP, Auth schema, role isolation, and PostgreSQL/PGMQ atomic rollback.

`make acceptance` runs the destructive behavioral suites only against a dedicated `*-identity-check` fixture, never the primary installation. See the implementation evidence for fresh-install, upgrade, concurrency, denial, mail, Files, export and recovery results.

For frontend layout iteration, run `LOOSKI_API_INTERNAL_URL=http://localhost:58888 pnpm dev` and open `http://localhost:58080`. Password login also needs an explicitly reachable `LOOSKI_AUTH_INTERNAL_URL`; use the Docker web service for the complete authenticated flow. That port is an explicit host development server; the packaged web service stays private on container port 3000. Native API iteration requires an explicit private database connection file and Auth URL; no database debug port is published automatically.

## API, CLI, and MCP

Installation metadata is public. `GET /api/v1/workspaces` requires a first-party Supabase access token and evaluates current workspace permissions. Browser cookies authenticate only the SvelteKit server, which calls Rust with a bearer token. The raw API does not accept cookie authentication.

```sh
curl http://localhost:58888/api/v1/system
cargo run --locked -p looski -- status
cargo run --locked -p looski -- mcp
cargo run --locked -p looski -- openapi
# Sign in through the browser, then use the stored login:
cargo run --locked -p looski -- login
cargo run --locked -p looski -- whoami
cargo run --locked -p looski -- workspaces
cargo run --locked -p looski -- logout
# Or supply an externally managed access token in a private file (never in argv):
LOOSKI_ACCESS_TOKEN_FILE=/absolute/path/to/token cargo run --locked -p looski -- workspaces
```

`looski login` uses authorization code + PKCE (S256) without a local callback. It prints a link to `/cli/authorize` on the installation named by `LOOSKI_API_URL`/`--api-url` (the public origin, default `http://localhost:58888`) and tries to open it; approve it in a browser signed in to that installation, then paste the one-time code (valid five minutes) back into the terminal. The installation mints a new Auth session for the CLI that is independent of the browser session: logging out of either leaves the other signed in. The session is stored per installation origin in the macOS Keychain, or elsewhere in a `0600` file under `$XDG_CONFIG_HOME/looski-suite/sessions/` (default `~/.config`). `LOOSKI_CREDENTIAL_STORE=file` forces the file store (on macOS under `~/Library/Application Support/Looski Suite/`), and `LOOSKI_CONFIG_DIR` relocates it. Access tokens refresh automatically before expiry and after a rejected request, under a cross-process lock so CLI and MCP processes never reuse a rotating refresh token. `looski logout` revokes the CLI session and deletes it locally. Authenticated calls require HTTPS except on loopback.

The MCP command speaks JSON-RPC on stdin/stdout with diagnostics on stderr. It advertises system/workspace, Contacts, CRM, Tasks, Files, access administration, invitation, jobs and export tools, which call the same HTTP API as the CLI. A local MCP client can launch `target/debug/looski mcp` with `LOOSKI_API_URL=http://localhost:58888`; it uses the stored `looski login` session and asks the user to run `looski login` when there is none. `LOOSKI_ACCESS_TOKEN_FILE`, when set, takes precedence; the file is reread on each call so an external session manager can replace it. External OAuth tokens are explicitly rejected until scoped grants exist.

The generated HTTP contract is served at `/api/v1/openapi.json` and checked in at `contracts/openapi.json`. Run `pnpm api:generate` after changing endpoint metadata. `openapi-typescript` generates frontend types; `openapi-fetch` provides typed calls. Public metadata has no bearer security requirement; business operations declare bearer security and enforce current action/resource permissions inside Rust business operations.

## Layout

| Path                                    | Responsibility                                                                              |
| --------------------------------------- | ------------------------------------------------------------------------------------------- |
| `crates/looski/src/application.rs`      | Transport-independent facade over module-owned business operations and focused SQLx DAOs.   |
| `crates/looski/src/api.rs`              | HTTP routes, DTOs, health, and OpenAPI.                                                     |
| `crates/looski/src/client.rs`, `mcp.rs` | CLI/API client and local MCP adapter.                                                       |
| `apps/web/`                             | SvelteKit SSR UI and generated API types.                                                   |
| `supabase/migrations/`                  | Sole suite PostgreSQL migration history, applied by the pinned Supabase CLI.                |
| `infra/`, `compose.yaml`                | Pinned runtime images, upstream bootstrap, routing, migration job, and optional ClickHouse. |
| `scripts/`                              | Development setup, contract generation, and verification.                                   |
| `docs/`                                 | Canonical architecture, decisions, product/build plans, and implementation evidence.        |

Module-owned SQLx DAOs handle persistence. Business operations enforce authorization, optimistic versions, idempotency, atomic audit and enqueue; HTTP/CLI/MCP/web adapters do not own those rules.

## Infrastructure boundaries

The Auth-only subset uses upstream Supabase PostgreSQL/Auth images and bootstrap SQL from `self-hosted/v0.8.2`. Caddy provides the suite gateway and the small internal Auth gateway. We do not package Studio, PostgREST, Storage, Realtime, or Edge Functions. `/auth/v1/health` is the only externally routed Auth endpoint in this slice; administrative, token, and data routes remain closed publicly. SvelteKit calls private Auth token/logout routes for browser sessions; the local provisioning command calls the private admin API.

API and Auth entrypoints read private file-backed secrets before dropping to their non-root service users. The API copies its connection and JWT verification files into tmpfs with mode `0400`; secrets do not enter image layers or Compose environment output.

Postgres bootstrap creates a non-administrative `looski_runtime` login. Migrations grant only the table access required by current operations and local owner provisioning; no blanket future-table grants are made. The runtime cannot migrate the suite schema. Auth owns its own migrations; SQLx never runs schema migrations.

`LOOSKI_DIAGNOSTICS_MODE=local` starts the optional ClickHouse service. API/worker events use a bounded asynchronous queue, fixed redacted fields and an INSERT-only credential. No host ports are published; business readiness and writes survive diagnostic outages. `disabled` stops collection without affecting PostgreSQL audit. The measured ClickHouse process used about 677 MiB during testing; see [diagnostics](docs/diagnostics.md). There is no automatic retention purge.

`LOOSKI_TUNNEL_MODE=cloudflare` adds a Cloudflare Tunnel connector that serves `LOOSKI_PUBLIC_URL` over HTTPS without publishing another host port. Provision the tunnel and DNS route once, then keep its credential with the installation; see [Cloudflare Tunnel routing](docs/setup-contract.md#cloudflare-tunnel-routing). Public HTTPS routing and secure cookies have been validated; public-domain email callbacks still need an SMTP validation pass. `make smtp` records SMTP credentials privately. Hosted Supabase transfer and release/source-distribution packaging remain unverified. Worker retries and fencing, migration checksum drift enforcement, encrypted backup, isolated restore and failed-update recovery have local behavioral tests; see [recovery](docs/recovery.md).

## Canonical plans and license

Start with the [build plan](docs/build-plan.md), [architecture](docs/architecture.md), [decisions](docs/decisions.md), and [scaffold evidence](docs/scaffold-status.md), and [identity slice](docs/identity-slice.md). Original software is licensed under [AGPL-3.0-only](LICENSE); upstream components retain their own licenses. See [NOTICE](NOTICE). The [trademark policy](TRADEMARKS.md) is separate from AGPL permissions. [Dependency and distribution checks](docs/implementation/dependency-review.md) record the remaining public-release gates.

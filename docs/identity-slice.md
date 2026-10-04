# Identity and workspace implementation

October 12, 2026. First usable foundation slice; not completion of phase 0/1.

## Scope and contract

- Password sign-in goes from a same-origin SvelteKit form to private Supabase Auth. Access/rotating refresh tokens live in host-only, HttpOnly, SameSite=Lax cookies; Secure follows the configured HTTPS origin. No token is serialized into page data or browser storage. Access cookies expire with the token; refresh cookies have a rolling 30-day browser lifetime. All responses disable caching. Logout revokes the current refresh session when Auth is reachable and always clears local cookies; failure is reported on the login screen.
- Rust verifies the selected installation's HS256 signature, exact issuer, audience, expiry, subject, session identifier, authenticated role, and non-anonymous status. External OAuth `client_id` credentials are rejected. Authentication then checks the account through upstream `/user`. It does not trust a stored identity mapping alone. Local HS256 is the current adapter; hosted asymmetric JWKS verification remains a future adapter, with no fallback between issuers.
- Supabase access JWTs can remain usable until expiry after sign-out; no claim of immediate access-token invalidation is made. Application user/membership/role/permission revocation takes effect on the next workspace query. Before sensitive mutations, select the necessary session freshness/revocation policy explicitly. [Supabase sessions](https://supabase.com/docs/guides/auth/sessions), [OAuth client claims](https://supabase.com/docs/guides/auth/oauth-server/oauth-flows).
- `GET /api/v1/workspaces` returns only workspaces for the verified issuer/subject, active user/membership/workspace, an active assigned role, and `workspace.read`. Authorization and data selection share one SQL statement/snapshot in the workspace DAO. A valid identity with no effective access gets an empty list. Invalid credentials get 401; unavailable Auth/database gets 503. Browser cookies alone never authenticate this endpoint.
- CLI `workspaces` and MCP `list_workspaces` use that exact HTTP contract. `looski login` acquires a separate first-party session through browser approval (authorization code + PKCE, pasted code) and refreshes it automatically; see the [CLI login slice](implementation/cli-login-slice.md). `LOOSKI_ACCESS_TOKEN_FILE`, when set, still supplies an externally managed token, reread per request, and takes precedence. There is no secret tool argument or command-line token.

## Operator bootstrap

Run `python3 scripts/bootstrap_owner.py` locally after `make up`. Docker access to this installation is the operator authority. There is no bootstrap HTTP endpoint, anonymous first-visitor grant, or automatic signup. The command explicitly confirms the initial email on the operator's authority; it is not the future recipient-verification flow for staff invitations.

The command locks a permanent singleton installation row, creates an upstream Auth user through its admin API, and atomically inserts the application user/mapping, workspace, membership, Owner role, `workspace.read` assignment, audit event, and consumed bootstrap latch. Role names grant nothing by themselves. Owner currently has only the single implemented permission; new capabilities must introduce deliberate grants in later migrations/operations. Ordinary role presets and CRM visibility remain open decisions.

Auth provisioning and suite writes cannot share a transaction. If Auth succeeds but the suite transaction fails, retry with the same email/password: the command proves possession by signing in to the existing account before attaching it. It never resets passwords, attaches by email alone, or deletes the Auth account as compensation. A completed bootstrap stays closed even if memberships/users become inactive. Recovery and additional staff/workspace creation need separate future operations; do not clear the latch to reopen setup.

The API process uses a restricted database role; only the local operator command performs initial permission writes. The JWT verification secret is a privileged shared signing secret in this local deployment, mounted as a file and handed to the unprivileged process through tmpfs. Never expose that file, the admin API, or database credentials to browser/CLI/MCP callers.

## Migration boundary

`20261012000100_identity_workspaces.sql` is additive, updates readiness schema version to 2, and leaves upstream `auth` tables unchanged. Scoped role and membership FKs prevent cross-workspace assignments. Runtime has no schema DDL or audit UPDATE/DELETE grants. This minimal audit shape records the one bootstrap operation; add request/resource/system attribution with the first corresponding operation.

No general membership edit API exists yet. Before adding one, implement removed-membership role cleanup, last-administrator preservation, optimistic concurrency, and audit together. Invitations, optional application selection, Contacts, opportunities/deals, magic links, SMTP, Gmail, and external OAuth grants are deferred. No product decisions about CRM visibility or opportunity/deal cardinality are silently settled by workspace discovery.

## Validation

Run `make check test`, `pnpm build`, `cargo build --locked`, and `make smoke` on the default development stack. The additional real integration test must use its own installation:

```sh
export COMPOSE_PROJECT_NAME=looski-suite-identity-check
export LOOSKI_HTTP_PORT=58890
export LOOSKI_PUBLIC_URL=http://localhost:58890
make up
python3 scripts/identity_smoke.py
make stop
```

The test retains only its own private fixture and test volumes for repeat runs. On a fresh fixture it injects an audit failure after Auth provisioning, verifies suite rollback, rejects a retry with the wrong password, and races two correct retries to establish exactly one owner. It also checks bootstrap replay and audit, real password login, same-origin protection, SSR, refresh/logout, raw API cookie rejection, authenticated CLI/MCP access, current authorization after revocation, and cross-workspace role constraints. Unit HTTP tests cover incorrect signatures, issuer/audience, expiry, service/anonymous/delegated credentials, and dependency outages. The primary installation is never given a test owner.

Validation passed locally on Apple Silicon: formatting/Clippy, four Rust tests, four setup guard tests, Svelte checks without warnings, contract drift, production builds, main-stack smoke, and the fresh-installation recovery/concurrency test. The standards review found remote HTTP credential exposure and bootstrap SQL placement; both were fixed and re-reviewed with no remaining findings. The spec review found no gaps within this increment. CI now includes the identity integration test but has not run on GitHub. The two isolated test installations are stopped with their test data retained; the primary stack remains available on port 58888.

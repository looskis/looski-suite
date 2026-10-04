# CLI/MCP login checkpoint

`looski login` signs the CLI and local MCP adapter in through browser approval: authorization code + PKCE (S256) with a pasted `code#state`, no localhost callback. The SvelteKit `/cli/authorize` page requires a browser session and returns to itself after password sign-in through a validated same-origin `next` path. It shows the installation origin, the signed-in email and the device-reported label, and Approve/Cancel are same-origin POSTs. The one-time code appears only in the no-store approval response. Email-link sign-in still lands on Workspaces; reopen the printed link afterwards.

Migration `20261012002800_cli_login.sql` (schema 29) adds `suite.cli_authorizations`, which stores only code hashes plus challenge, label, expiry and outcome, and the append-only, user-level `suite.security_events`. Runtime receives SELECT/INSERT and column-limited UPDATE on authorizations and INSERT on events. Rows are retained as sign-in history; there is no purge yet. Recovery accepts schema 29.

HTTP contract, tagged `cli`:

- `POST /api/v1/cli/authorizations` (bearer, used by the web page): creates a five-minute code for the active mapped user; at most five can be outstanding (429). Users without an application account get 403. It is not idempotent by design.
- `POST /api/v1/cli/sessions` (public): exchanges `{code, code_verifier}` exactly once. Wrong verifiers and inactive accounts consume the code. A new Auth session is minted through private admin generate-link (magic link, nothing emailed) and `/verify`.
- `POST /api/v1/cli/sessions/refresh` (public): proxies rotation, only for sessions recorded as CLI-issued.
- `POST /api/v1/cli/sessions/logout` (bearer): revokes only the presented session.

Failures use `invalid_grant`. Public endpoints are limited per forwarded client address to 20 exchanges and 60 refreshes per minute. Re-running the smoke suite within a minute can therefore hit the exchange limit.

CLI: `login [--device-label] [--no-browser]`, `logout`, `whoami`. Business commands and `mcp` use `--token-file`/`LOOSKI_ACCESS_TOKEN_FILE` when given, otherwise the stored login for the `--api-url` origin. Storage is the macOS Keychain (`security-framework`, already in the lockfile), or elsewhere a 0700 directory with a 0600, atomically replaced file. `LOOSKI_CREDENTIAL_STORE=file` and `LOOSKI_CONFIG_DIR` override this. Refresh happens two minutes before expiry and once after a 401, under an exclusive per-origin `std::fs::File` lock with a reread. A revoked refresh deletes the stored login. Unauthenticated CLI/MCP errors are structured `unauthenticated` failures that tell the user to run `looski login`; CLI exit status is 4.

Provider facts verified against pinned Auth v2.196.0: generate-link for an unknown email would create a user, so Rust reads the user by subject first. Generating the link replaces any outstanding recovery/magic-link token for the account. Auth accepts the immediate parent of the active refresh token indefinitely, as protection against a lost response. Only older replaced tokens fail after the 10-second reuse interval, and that failure revokes the whole family. Logout with a deleted session returns `session_not_found`, so the API rejects that access token on its next `/user` check.

Validation on Apple Silicon: `make check` and `make test` passed, including new Rust tests for the RFC 7636 vector, throttling, private file storage, origin separation, lock exclusion and login guidance. The isolated `looski-cli-login-identity-check` fixture (port 58897) was upgraded from schema 28 to 29. On it, `scripts/cli_login_smoke.py` passed:

- login return path and off-origin `next` rejection, CSRF and invalid requests;
- full exchange, reuse, wrong verifier, expiry, suspension and hashed-only storage with audit;
- refresh rotation and replaced-token rejection, and refusal to refresh browser sessions;
- the real `looski login` prompt with pasted code, and foreign state rejection;
- private storage, `whoami`, CLI and MCP with stored credentials;
- six concurrent CLI/MCP refreshes without token reuse, and 401 refresh-retry;
- logout revoking the CLI but not the browser session;
- membership removal denying a live login, and the approval bound.

The complete `make acceptance` suite, now including this smoke, then passed on the same fixture. A manual run confirmed Keychain storage, `whoami` and Keychain deletion on logout with the default macOS store. Unsigned rebuilt binaries may trigger a Keychain access prompt; non-interactive MCP hosts should use the file store if that is a problem. HTTPS/Cloudflare, Linux file-store runs and hosted Auth were not exercised.

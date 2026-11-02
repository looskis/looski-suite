# Local CI preflight

Run this before pushing a candidate revision to avoid spending a remote run on
local configuration mistakes:

```sh
python3 scripts/ci_local.py
# Or check a specific committed candidate on another free port:
python3 scripts/ci_local.py --ref HEAD --port 58891
```

The preflight snapshots the specified **commit**, excluding uncommitted edits,
`.env`, local secrets, dependencies and existing databases. It checks the exact
Node, pnpm and Rust versions against `.github/workflows/check.yml`, then runs its
dependency installation, `make check test`, frontend build, Rust build, fresh
Docker installation and full acceptance suite in order. It refuses to run if the
workflow command list or environment keys have changed without updating the runner.

Each invocation gets a new `*-identity-check` Compose project and fresh volumes.
Mail is configured from the workflow; diagnostics use the fresh-install default
and the public tunnel is disabled. The port and public origin change together.
Existing development and acceptance installations are left alone. A lock prevents
two preflights from using this checkout's Cargo cache concurrently. Build and
dependency caches are reused; fixture data is not.

Logs, step durations, tool versions, the exact revision and the result are saved
under `.local/ci-preflight/<timestamp>-<revision>/`. Containers are stopped on
success or failure. The snapshot, private fixture files and stopped volumes are
retained for diagnosis. They can consume substantial disk space; review the project
identity before removing old preflight resources. No remote workflow is triggered.

Prerequisites are a running Docker engine with Compose, Python 3.9+, and the
workflow's pinned Node, pnpm and Rust tools. The runner prefers tools installed in
`.local/ci-tools/node-v<VERSION>-<PLATFORM>-<ARCH>/bin`,
`.local/ci-tools/pnpm/node_modules/.bin`, and `~/.cargo/bin`, then checks PATH.
It never upgrades global tools.

On macOS with the default Colima context, Colima may forward published ports only
over IPv4. When the host IPv6 listener is missing, the helper verifies the declared
Docker IPv6 binding and probes the real IPv6 gateway inside the VM. It then starts
a temporary SSH forward from host `::1` to that VM IPv6 endpoint. This adaptation
is recorded in the result and removed after the run. The dual-loopback acceptance
check still runs unchanged; missing Compose bindings or an unhealthy VM endpoint
are failures. Other Docker contexts receive no automatic forwarding workaround.

This is a local configuration and behavior preflight, **not proof of GitHub runner
parity**. Host checks and CLI tests run on the host OS/architecture. Application
containers use the production Dockerfiles, with the local Docker architecture.
Python, Docker, Compose, filesystem permissions, architecture, network access and
GitHub Actions setup/cache behavior may differ from Ubuntu on x86. Matching those
requires an isolated Linux x86 VM/runner running the same workflow; emulation on
Apple Silicon can be slower. A remote run is still the final confirmation.

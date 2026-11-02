# Install Looski Suite with an agent

Looski is a self-hosted development preview. The supported install path below works from the public source repository: Docker builds and runs the complete stack. You do not need host Rust, Node, pnpm, Make, a domain, or a Cloudflare account for a local installation.

Copy this entire prompt into a terminal-capable agent:

```text
Install and start Looski Suite for me. Follow https://raw.githubusercontent.com/looskis/looski-suite/main/INSTALL.md.

Use this computer unless I specify an existing server. Inspect the environment, set up missing prerequisites where you can, and run the supported installation. Keep going through build, startup, and verification; do not stop at a plan or a list of commands for me to execute.

If I provide a public hostname, configure Cloudflare Tunnel and DNS using my authorized account. Otherwise get the local application running first. Ask only for missing access, unavoidable administrator/login steps, or choices involving purchases or existing infrastructure.

Return the working application URL and my private owner setup link so I can choose my password in the browser. Preserve progress if interrupted, and clearly identify anything that still needs my action.
```

## Default installation: source checkout

Use this path now. Do not wait for a binary release or substitute an example version number. The first build can take several minutes; keep following its progress until the command finishes. Internet access is needed to download dependencies and images.

1. Inspect the host and existing Docker runtime. Use an existing suitable runtime rather than installing a second one. Required: macOS or Linux on ARM64/x86-64, Git, Python 3.9+, a running Linux Docker engine and Compose 2.20+.
2. If prerequisites are missing, follow the official [Docker installation instructions](https://docs.docker.com/engine/install/) for the detected Linux distribution, or [Docker Desktop for Mac](https://docs.docker.com/desktop/setup/install/mac-install/). Handle package installation and startup within the user's authorization. Return any unavoidable admin/login/terms interaction to the user with the exact remaining step, then resume. Do not replace an existing runtime or silently change the active Docker context.
3. Choose an unused installation directory (default `~/looski-suite`). If it exists, inspect it and resume that installation; do not clone over it, delete it, or blindly pull updates into an installed release. For a new installation:

   ```sh
   git clone --depth 1 https://github.com/looskis/looski-suite.git "$HOME/looski-suite"
   cd "$HOME/looski-suite"
   ./looski-admin doctor
   ./looski-admin install
   ```

4. Fix any reported prerequisite or port conflict and retry the same installation. `doctor` includes `next_actions`. Once startup completes, run `./looski-admin verify --wait 180`. If the owner is pending and no valid link was returned, run `./looski-admin claim`.
5. Read the returned `private_link_file` and give its complete URL to the intended owner in the private conversation. Open it for them if a suitable browser is connected. They choose their own account details and password. Never create an arbitrary production owner or ask for their password in chat.

Use an agent tool that supports long-running commands, preserve its process/session ID, and continue checking output. A tool returning a running process is not a failed installation. Do not start a second installer while the first is active. On a later conversation, inspect the existing directory with `./looski-admin status`; use `start`, `verify`, or `claim` as needed.

For SSH installs, execute these commands on the server. A localhost URL belongs to that server, not the user's laptop. Configure the requested public hostname before handing it off, or explicitly arrange an SSH local port forward for a private evaluation. Do not report a server-local URL as directly reachable from the user's browser.

## Gather only missing information

Determine whether the target is the current Mac/Linux machine or an existing Linux server accessible through SSH. For a public installation, determine the requested hostname. The installer discovers its Cloudflare account and active zone from the authorized token; `--account-id` and `--zone-id` are optional selectors when needed. The installer itself uses a private scoped API token file, not a built-in OAuth connection.

If a machine is not available, an agent with an authorized hosting integration may provision one within the user's explicit provider/budget instruction, then follow the Linux path. This installer does not purchase servers or domains. Do not infer spending authorization from the installation prompt. Unsupported registrar delegation, login, MFA and OS administrator steps are resumable human handoffs. Record the exact remaining action; do not treat DNS propagation as a failed installation or silently switch addresses.

Prerequisites: macOS or Linux, Python 3.9+, a running Linux Docker engine and Compose 2.20+. Release installation needs no Git, Make, Rust, Node or host cloudflared. Inspect before installing system dependencies; use the target OS's supported installation process and respect existing runtimes. The machine must stay awake and running to serve users. No fixed memory/disk minimum has yet been validated; `doctor` reports free disk and actual runtime readiness.

## Install and resume

```sh
./looski-admin doctor
./looski-admin install --project looski-suite --port 58888
```

All admin commands emit one JSON result on stdout and progress on stderr. Exit 0 means the command succeeded, 2 means readiness checks did not pass, and 1 means an operation failed. Preserve the installation directory, `.env`, and `.local/`; identity and credentials belong to this installation. Never change the project name to work around a failure. Retrying install uses existing secrets and volumes and executes the migration job again. Never delete volumes, rotate keys, or replace DNS records to make setup pass.

If the chosen local port is occupied, retry `install --port <free-port>`; the project and authentication identity are preserved. For a second installation, use a separate directory, project and port. The installer ignores ambient installation overrides and persists its explicit choices. Do not run two installers concurrently in the same directory.

For local evaluation, the result points to a private `owner-setup-link` file. Read that file only to deliver the link directly to the intended owner. It is a bearer credential, expires after one hour, and belongs only in the private handoff—not a log, issue, shared document, or status report. The owner enters their password in the browser; never ask them to paste it into agent chat. The permanent database latch prevents a second owner bootstrap even if the installer is rerun. To replace an expired link, run `./looski-admin claim`. The ordinary status/verify output contains no token.

## Publish through Cloudflare

Use an account-scoped Tunnel Edit token plus Zone DNS Edit and Zone Read for the selected zone. Store it as a regular mode-0600 file outside the checkout; pass its path, never its contents, in argv. Do not paste it into chat. The zone must already be active. This flow accepts a free subdomain, not the zone apex.

```sh
./looski-admin publish \
  --url https://suite.example.com \
  --token-file /private/path/cloudflare-api-token
```

This creates a remotely managed tunnel, routes it to the private gateway, creates a proxied CNAME, saves the connector token privately, restarts the stack and checks the public origin. The Cloudflare management token is not retained. Outbound Cloudflare connectivity is required. Existing conflicting DNS or externally changed tunnel routes are reported without replacement. A private journal lets retries recover a successful creation even if the response or process was interrupted. Existing locally managed tunnels require explicit migration and are never adopted automatically.

The public URL controls browser callbacks and cookies. The installer preserves the original Auth issuer when promoting a local installation so user identity links and workspace access remain stable. Users use the public hostname after promotion. Existing bookmarks and provider callback configuration may need updating.

Publishing waits for readiness for up to three minutes. If public verification has not passed yet, resume with `./looski-admin verify --wait 180`; once it passes, use `./looski-admin claim` if the owner is not created. Do not claim the public deployment is ready merely because the connector started.

Cloudflare's API workflow: https://developers.cloudflare.com/tunnel/get-started/.

## Email and backups

Email is optional for local evaluation. For public operation, configure an existing SMTP provider using a mode-0600 JSON file containing `host`, `port` (465, implicit TLS), `user`, `password`, `from`, and `sender_name`:

```sh
./looski-admin smtp --credentials-file /private/path/smtp.json
```

Then verify an actual sign-in/recovery message and callback with the owner using [the email guide](docs/auth-email.md). SMTP configuration alone is reported as `configured_unverified`; it is never reported as successful delivery. Provider signup, sender verification and DNS records require the selected provider's authorized integration or a human handoff.

Encrypted snapshots use the existing recovery tools:

```sh
python3 scripts/recovery.py build-tools
python3 scripts/recovery.py keygen --output /private/off-installation/recovery-key
./looski-admin backup --recipient AGE_PUBLIC_RECIPIENT --output /backups/looski-first.age
python3 scripts/recovery.py verify --archive /backups/looski-first.age --identity /private/off-installation/recovery-key
```

Keep the recovery identity separately from the installation and backup archive. Follow [recovery](docs/recovery.md) for an isolated restore rehearsal. Backups are manual unless an operator separately configures a scheduler and destination; never report daily backups as configured by default. Updates must follow the existing verified-snapshot/recovery procedure; automatic release switching is not implemented in this preview.

## Verify and hand off

Run `./looski-admin verify` and `./looski-admin status`. Readiness checks cover the actual configured origin, API/schema/Auth readiness, login page, matching canonical URL and closed administrative/data routes. A healthy login page does not prove an authenticated browser journey; have the owner finish their claim and sign in, then verify workspace access with them. Use an agent browser if one is available and the user has authorized it.

Return the application URL, private owner link if pending, host/project identity, verified checks, and explicit email/backup/pending prerequisite status. Do not include secrets. Example:

> Looski is running at https://suite.example.com. HTTPS and service readiness passed. Use this private link to create your owner account. Email delivery is awaiting verification; backups have not been configured.

`start`, `stop`, `status`, `verify`, `claim`, `smtp`, `publish`, and `backup` remain available through `looski-admin`. Stopping preserves data. No reset or delete-data command is provided.

## Optional prebuilt releases

Prebuilt releases are an optional future fast path. Use one only when an explicitly published, validated preview exists; otherwise use the default source installation above. Do not invent a version or download URL. The release workflow currently prepares drafts.

For a published preview, download `scripts/download_release.py` from its reviewed source revision and run it with the actual `--version` and a fresh `--destination`. It verifies SHA-256 and refuses unsafe archives or existing directories. Supply `--sha256` for an independently obtained digest; verify the artifact with `gh attestation verify <archive> --repo looskis/looski-suite` when available. Release mode pulls pinned prebuilt images instead of building.

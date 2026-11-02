# Agent installation preview

Implemented November 1, 2026. [INSTALL.md](../../INSTALL.md) is the user/agent entry point. This change prepares a distribution preview; it does not publish a release or certify external-provider behavior.

## Delivered behavior

- `looski-admin` offers prerequisite checks, install/resume, start/stop, structured status, actual-origin verification, expiring owner claim, Cloudflare publishing, SMTP configuration and encrypted snapshot creation. Command output separates JSON results from progress. Installation identity, core secrets and data survive retries; explicit port correction preserves the issuer and project identity.
- Releases use prebuilt ARM64/AMD64 images pinned by digest, a complete Compose recipe, corresponding repository source, SHA-256 checksums and GitHub provenance attestations. Packaging rejects images without both platforms. The manually dispatched release workflow prepares a draft preview. The downloader checks integrity, rejects traversal/links/duplicate members, and refuses existing destinations.
- Cloudflare provisioning records intent before external mutations, reconciles interrupted tunnel/DNS creation, and refuses occupied hostnames or externally changed ingress. Management credentials are read from a private external file; only the installation connector token persists. Existing locally managed tunnels remain supported separately.
- The browser owner claim uses a one-hour, 256-bit bearer link in the URL fragment. The separate setup service receives only its hash, requires the configured Origin for submission, bounds request size and valid-token attempts, returns no-store/no-referrer/CSP headers, and shares the permanent transactional bootstrap latch. No claim is active by default under `make up`. Recovery excludes claim credentials and quiesces the setup service with the other writers.
- `LOOSKI_AUTH_ISSUER` separates persistent authentication identity from the public browser origin. API, Auth, setup, invitations and notification delivery use the same configured issuer. Moving from local to public routing preserves existing user/workspace mappings without rewriting them.

## Verification

Local checks passed on an Apple Silicon host with a Linux Docker engine:

- 39 Python tests, including provider failures after remote commit, repeated provisioning, DNS conflicts, changed routes, private token-file guards, release archive integrity/path checks, image pinning, port correction, issuer persistence and interrupted first-secret publication.
- 41 Rust tests, formatting and Clippy with warnings denied; setup token expiry and closed/cross-origin entry points are covered.
- Fresh dedicated `looski-installer-identity-check` installation on port 58971. HTTP acceptance checks reject absent/wrong claim tokens and cross-origin submission, race two correct claims and observe exactly one owner, reject replay, sign in through the web flow, and retrieve the single authorized workspace.
- The same installation changes browser origin from localhost to 127.0.0.1 while retaining the original issuer. Password sign-in and workspace access still succeed. This tests origin/identity separation locally; it is not a substitute for the public HTTPS/provider callback pass.
- Installer rerun retains core secret digests and the claimed owner, and checks required container health, the completed migration job, readiness, canonical origin and closed admin/data paths.
- Generated digest-only Compose configuration validates. Registry manifest resolution was exercised against Caddy. The managed connector reads a mode-0600 mounted token with its restricted container configuration and rejects a dummy credential without networking.

The fresh installation acceptance is wired into its own GitHub Actions job. GitHub CI has not been run by this task. No browser connection was available for visual layout inspection; the sign-in checks exercise the real HTTP/cookie/SSR flow.

## Remaining release checks and scope

Run real scoped Cloudflare provisioning/resume/conflict checks, external SMTP delivery and callbacks, clean prebuilt-archive installs on both architectures, anonymous registry pulls, full image/third-party-notice review, and encrypted restore validation before publishing a draft. The full release build/push/attestation workflow was not executed locally.

Hosting purchases, registrar changes, provider signup, automatic backup scheduling and automated release switching are not built into this preview. The guide provides explicit agent/human handoffs and uses the existing recovery workflow for upgrades. Status never equates configured SMTP with verified mail, or a healthy login page with a completed owner sign-in.

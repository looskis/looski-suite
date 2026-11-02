# Communications implementation checkpoint

November 2, 2026. Owning module: `communications`. The user-visible journey is connect an operator-provisioned account → receive a message → link a contact → reply → observe delivery state, with independently authorized contact history.

Implemented: schema 37; private-by-default account access; explicit shared-account visibility; module selection and disablement; native Blueski adapter; versioned external HTTP connector; durable normalized events, replay cursors and provider-message deduplication; transactional outbound intent/audit/PGMQ scheduling; current worker authority checks; provider identity/idempotency preflight; contact links and CRM-composed timeline; scoped business exports; web, HTTP/OpenAPI, CLI and MCP interfaces. Setup mounts a private connector configuration file into API and worker. Backup captures it; restore pauses accounts and quarantines jobs.

Validation:

- `make check test`: Rust formatting, strict Clippy, frontend formatting/type checks, generated-contract drift, 44 Rust tests and 42 Python tests passed.
- `scripts/communications_smoke.py`, with `COMMUNICATIONS_BROWSER=1`: disposable copy of a dedicated real PostgreSQL fixture, source schema 36 → 37, runtime role grants, signed events, invalid signatures/installation identity, exact destination filtering, conflicting event replay, semantic duplicate messages, out-of-order webhook plus polling gap recovery, pagination, contact links, timeline/export access, concurrent idempotent queue requests, accepted/delivered receipt ordering, repeated dispatch after simulated lost completion, and the independent email HTTP adapter.
- The same journey verifies CLI/MCP parity, private-account invisibility despite workspace grants, cross-workspace denial, module disablement without automatic job resume, revoked authority at worker execution, refusing pre-0.2 Blueski before sending, and rejecting incoming events after workspace suspension.
- Rendered Svelte inbox, conversation/reply form, escaped message text and linked-contact page passed through a local frontend against the same API. Fixtures use fake transports; no messages are sent to real recipients.

The setup and connector contract are in [Communications](../communications.md). Real Blueski/macOS delivery is not claimed until a reachable authenticated installation is configured and an explicit test recipient selected. Email validation uses a connector fixture; Gmail OAuth and IMAP are not implemented. New-conversation composition, call sessions, outbound attachments, automated responses and Helpdesk conversation linking remain future work. The new schema/configuration recovery paths are implemented; a full encrypted snapshot/restore rehearsal for this slice remains separate from the disposable database journey.

Development deployment: schema 37 applied through the existing migration runner; API, web, gateway, worker and tunnel reported healthy at `https://preview-suite.loo.ski`. No real connector was configured.

# Local backup and recovery

The manual recovery runner creates encrypted, portable installation snapshots. It does not schedule backups, delete old archives, copy them off-machine, or switch public routing. Those choices remain with the installation owner. A snapshot on the same drive does not protect against drive loss; keep both an encrypted archive and a separately protected decryption identity recoverable outside that machine.

## Create and verify

Run from the installation checkout, with the same `COMPOSE_PROJECT_NAME`, port, and public-origin settings used by setup. Docker must be running. Build the pinned encryption helper once:

```sh
python3 scripts/recovery.py build-tools
python3 scripts/recovery.py keygen --output /private/recovery/identity
python3 scripts/recovery.py create --recipient age1... --output /private/backups/snapshot.age
python3 scripts/recovery.py verify --archive /private/backups/snapshot.age --identity /private/recovery/identity
python3 scripts/recovery.py status
```

Create the destination directories first. The identity file must be a regular mode-0600 file. Key generation prints the public recipient and identity-file location, never the private key. Existing output files are never overwritten. An archive is encrypted with age 1.3.2; the identity is never included in that archive. Loss of that identity makes the archive unusable.

The runner shares setup's installation lock, freezes the recovery recipe and checks applied migration hashes, captures exact service images (including stopped services), stops currently active application/Auth writers, captures data, verifies and encrypts the package, then starts exactly the writer containers that were previously running. Backups require every core service to have been created at least once, including `auth-templates`. Supported database checkpoints are schema 23 and schemas 28–34; arbitrary historical deployments are not claimed. Full healthy installations wait for restored service health; intentionally partial installations preserve their running/stopped set. `status` records maintenance duration and failure. If a process was killed before cleanup, run `recover-services` before starting another snapshot. The same private database must not be modified through an independent administrative connection during capture.

Format 5 contains the PostgreSQL custom dump, complete role attributes and exact membership/grant options, explicit PGMQ metadata/live/archive binary data and sequence state, Auth/business/audit/migration/idempotency records, actual Files bytes, installation secrets, frozen source/deployment inputs, migration checksums and compressed pinned Docker images. The raw ClickHouse diagnostic history is excluded; PostgreSQL audit remains included. Image payloads make recovery independent of registry tags and the original Docker cache. Unique archive tags map the source manifest/index identity to the config identity used by classic Docker engines; restored services are pinned to the verified local image ID. Use the same supported CPU architecture; cross-architecture recovery is not claimed. Versions 1–4 were incomplete development experiments and are rejected.

The knowledge base needs no extra step: its collections, member lists, documents and revisions are in the PostgreSQL dump and covered by the table-count check, and its search index is a generated column restored with them.

Projects needs no extra step: its boards, comments and recipe runs are in the PostgreSQL dump and covered by the table-count check. A queued deal conversion restores with its durable job, and it still cannot produce a second project.

CMS needs no extra step. Its tables are in the PostgreSQL dump and covered by the table-count check, media bytes are ordinary Files bytes covered by the hash check, and public keys survive, so a restored page keeps its address. To undo an unwanted edit without a restore, use revision recovery in the application: restoring a revision copies it into the draft and publishing makes it live. Rehearsal evidence is in the [CMS checkpoint](implementation/cms-slice.md).

## Restore and rehearse

Use a fresh project name and unused loopback port:

```sh
python3 scripts/recovery.py restore --archive /private/backups/snapshot.age --identity /private/recovery/identity --project looski-suite-recovered --port 58889
python3 scripts/recovery.py preview --target /absolute/path/to/checkout/.local/recoveries/looski-suite-recovered
```

Restore never replaces an existing installation, target directory, container, or volume. It authenticates the complete ciphertext and verifies every archive member before extraction or database writes. Private temporary data is removed after successful recovery; a failed isolated target is retained for investigation. Recovery reports are mode 0600 under the target directory. They include the source snapshot time, verified counts and quarantined jobs, but no passwords or bearer tokens.

The database is restored first on an internal network. The runner checks table counts, migration source/history, exact role memberships, every file hash/length, and byte-for-byte PGMQ data. It rotates Auth signing material, removes sessions, refresh tokens and pending one-time Auth links, keeps original identity issuer mappings, and quarantines unfinished jobs. Password login establishes new sessions without matching Contacts by email. Existing invitation links tied to the old signing material are invalidated.

Preview exposes a loopback-only gateway. Only that gateway joins an ingress bridge; Auth/API/PostgreSQL remain internal. Outbound mail, diagnostics and workers are disabled. Restoring a snapshot does not authorize replaying external effects: compare quarantined work with what happened after the snapshot, then explicitly resume only reconciled jobs. No production promotion command or automatic tunnel/DNS change is provided. Review the recovery report and run the CRM acceptance journey before planning a public cutover.

## Update recovery

Create and verify a snapshot before `make up` updates an existing installation. Setup builds immutable migration images and stops business writers before applying changes. The Supabase CLI remains the only migration runner/history. An accompanying checksum ledger records candidate source before application; applied versions are immutable, while an unapplied failed migration can be corrected. The initial adoption trusts the shipped baseline for the original 28 migrations. Subsequent checks bind database-applied versions to recorded hashes. This does not attest to arbitrary historical manual SQL changes.

If an update fails, callers stay stopped instead of running incompatible application code. Preserve the failed installation, fix an unapplied migration when appropriate, or rehearse the pre-update snapshot in a separate installation. Do not remove or rewrite applied migration rows to force an update. Recovery does not erase post-snapshot data loss; the snapshot timestamp is the recovery point. Snapshot creation and update are separate commands, so stop new business traffic between them when that recovery window matters.

See the [implementation evidence](implementation/recovery-slice.md) for measured fixture results and remaining release boundaries.

Images are captured by the ID each container runs. `scripts/dev.py` disables default build attestations so an unchanged rebuild keeps that ID; a direct `docker compose build` with attestations can orphan a running container's ID on the containerd image store, and a backup then fails safely, leaving the installation running. Rerun `make up` to recover.

Schema 34 adds Helpdesk. Tickets, immutable messages, private notes, reference links, hashed customer access codes, revocations and retry/event records are in the coordinated PostgreSQL snapshot; there is no additional storage or worker. Ticket codes survive restore, so the restored installation must remain isolated until cutover. Staff Auth sessions still follow the existing recovery rules. The recovery rehearsal checks staff history and a saved ticket code against the separate customer projection.

Schemas 35–36 add time entries/budgets and appointment types, reservations, versioned-intake associations, lifecycle events, reminder generations and durable job due times. These live in PostgreSQL and need no new storage service. The isolated rehearsal verifies exact duration, a budget, saved booking code, original intake version/answers and reservation history. Restored pending reminders are quarantined along with other pending jobs; workers and outbound mail remain disabled until explicit review. A running timer continues measuring wall-clock elapsed time across an outage or restore; stop or correct it if that is not the intended work duration.

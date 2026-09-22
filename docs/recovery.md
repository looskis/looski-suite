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

The runner shares setup's installation lock, freezes the recovery recipe and checks applied migration hashes, captures exact service images (including stopped services), stops currently active application/Auth writers, captures data, verifies and encrypts the package, then starts exactly the writer containers that were previously running. Backups require every core service to have been created at least once, including `auth-templates`. Supported database checkpoints are schema 28 and the prepared schema-23 installation used for this update; arbitrary historical deployments are not claimed. Full healthy installations wait for restored service health; intentionally partial installations preserve their running/stopped set. `status` records maintenance duration and failure. If a process was killed before cleanup, run `recover-services` before starting another snapshot. The same private database must not be modified through an independent administrative connection during capture.

Format 5 contains the PostgreSQL custom dump, complete role attributes and exact membership/grant options, explicit PGMQ metadata/live/archive binary data and sequence state, Auth/business/audit/migration/idempotency records, actual Files bytes, installation secrets, frozen source/deployment inputs, migration checksums and compressed pinned Docker images. The raw ClickHouse diagnostic history is excluded; PostgreSQL audit remains included. Image payloads make recovery independent of registry tags and the original Docker cache. Unique archive tags map the source manifest/index identity to the config identity used by classic Docker engines; restored services are pinned to the verified local image ID. Use the same supported CPU architecture; cross-architecture recovery is not claimed. Versions 1–4 were incomplete development experiments and are rejected.

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

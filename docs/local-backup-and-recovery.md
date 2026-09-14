# Local backup and recovery proposal

Status: proposed design, October 4, 2026. No backup implementation exists yet. Tested recovery is already a release requirement; the mechanism, schedule, retention, destination, and recovery targets below need acceptance and implementation evidence. This procedure covers restoring a local installation. Moving to hosted Supabase is a separate migration.

## What local storage provides

Docker volumes persist across ordinary container replacement and restart. They do not protect against deletion, drive loss, corruption, or a damaging application update. Self-hosting does not inherit the managed Supabase service's backup operations. The suite must provide backup and recovery tooling that an owner can operate without using database administration tools directly.

## Proposed initial backup

For small installations, start with a coordinated snapshot archive. Use PostgreSQL's custom-format logical dump and a tested restore recipe for the pinned database, Supabase services, and extensions. PostgreSQL supports consistent dumps of a running database; coordinating mutable files and other services is a separate concern. Logical snapshots are an initial scope choice, not a universal solution for large databases or short recovery windows. See [PostgreSQL SQL dumps](https://www.postgresql.org/docs/current/backup-dump.html) and [pg_dump limitations](https://www.postgresql.org/docs/current/app-pgdump.html).

Each backup includes:

- Application records, memberships, identity mappings, audit history, and migration state.
- Supabase-managed Auth data required to recover identities and credentials. Verify the dump includes `auth`; an application-schema export is insufficient.
- PGMQ queue contents and application workflow progress/idempotency records. Verify extension-specific dump and restore behavior rather than assuming extension creation restores queue data.
- Actual uploaded file bytes alongside their database metadata, including any configured Storage backend.
- A version manifest, checksums, deployment configuration, and a compatible database roles/grants/bootstrap recipe. A single-database dump does not include cluster-wide roles.
- Required recovery secrets in the encrypted archive or a separately recoverable encrypted bundle. Keep the archive's decryption key independently of the failed machine and the archive itself. Never commit secrets or backups to source control.

Proposed procedure: acquire a backup lock; enter maintenance mode; pause and drain application workers and stop all remaining writers, including Auth and file writes; leave PostgreSQL running; capture the database and frozen file data; encrypt and checksum the package; finalize it atomically; then restore normal service even if backup failed. Measure the maintenance window before making availability claims. Do not copy a live PostgreSQL volume as an ordinary file backup.

The backup/restore runner must work independently of the application job queue. Recovery cannot require the broken database to dispatch a PGMQ job. An installation-level scheduler can invoke the runner while the host and Docker runtime are available.

## ClickHouse log recovery

Local ClickHouse is now part of the deployment. Include its configuration, pinned image information, and collector configuration in recovery metadata. The PostgreSQL archive already covers authoritative business audit records; a ClickHouse copy must not be their only backup. Raw ClickHouse log data needs a separate backup/retention decision and compatible procedure if historical diagnostics must survive machine loss. The logical PostgreSQL dump does not capture ClickHouse. Validate restarting the recovered suite with a fresh log store and disclose any loss of diagnostic history; do not silently claim a full log-history restore.

## Schedule and protection against machine loss

Proposed starting policy: daily backups and a backup before each upgrade, retaining seven daily and four weekly snapshots. Copy encrypted backups to an owner-selected destination outside the Mac, such as another machine, an external drive, or object storage. A backup on the same drive alone does not cover drive loss. Rotation must not delete the last usable backup when a new attempt fails.

Expose the last successful backup, latest verified external copy, last restore rehearsal, and failures. A sleeping or offline Mac can miss its schedule; report stale protection and run missed work after wake rather than claiming a backup happened.

With successful daily backups, the intended data-loss window is up to approximately 24 hours. The actual window is the age of the latest usable copy; failed or missed runs extend it. Recovery duration must be measured against realistic database and file sizes. If that loss window is unacceptable, evaluate physical base backups and continuous WAL archiving for point-in-time database recovery, plus compatible file recovery. That is additional operational scope, not something enabled by Docker or PGMQ. See [PostgreSQL continuous archiving](https://www.postgresql.org/docs/current/continuous-archiving.html).

## Restore procedure

1. Select a complete backup and verify its manifest, integrity, decryption, and supported versions. Preserve the current installation for investigation or rollback.
2. Start a compatible database and services with fresh storage in an isolated recovery deployment. Keep public routing, ordinary users, outbound delivery, and workers disabled.
3. Restore required roles/extensions, database contents including Auth and queues, files, and configuration using the tested ordering. Do not restore over an actively used database.
4. Check identities, workspace membership, permissions, contact relationships, file access, queue counts, and workflow progress. Require administrator authentication before recovery actions. Validate the session invalidation/re-authentication policy before public release.
5. Review work that may have executed after the snapshot. An older database can forget an email or other external action that already happened. Reconcile against provider idempotency keys or external records; quarantine uncertain jobs rather than blindly replaying them.
6. Run the CRM acceptance journey, switch routing to the recovered installation, and explicitly resume approved jobs. Record the recovered timestamp and measured data loss/downtime.

## Release evidence

Restore a representative encrypted archive on a clean supported installation. Demonstrate complete Auth/business/file/queue recovery, an interrupted backup that leaves the previous archive usable, missing-file detection, failure visibility, and safe handling of queued external effects. Test recovery from an upgrade failure with compatible images and schema state. Publish observed recovery time and limitations; a successful archive creation alone does not establish recoverability.

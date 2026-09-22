# Local diagnostics implementation

Provide a bounded asynchronous diagnostic pipeline from the API and worker to the pinned local ClickHouse service. PostgreSQL transactional audit remains authoritative. Diagnostic loss or an unavailable collector/store must never fail, delay, or authorize a business operation.

Emit a fixed event schema: event ID, request ID when present, service, event kind, framework route template or bounded worker outcome, HTTP status, elapsed milliseconds, and time. Never forward arbitrary tracing messages, SQL errors, input paths, query strings, headers, bodies, tokens, contact fields or uploaded bytes. The framework generates request IDs; incoming IDs are not trusted. Route templates retain placeholders rather than business IDs.

Use a 1,024-event nonblocking channel per process, batches of at most 64, one-second flush, two-second HTTP timeout, and no disk spool or unbounded retries. Failed batches and overflowing events are dropped with bounded aggregate diagnostic warnings. This is deliberately lossy operational evidence; do not use it for compliance history. Keep the HTTP ingest account restricted to INSERT on the single diagnostic table and publish no ClickHouse host ports. Local administration uses the private container interface; no business API exposes raw diagnostic queries.

The local mode is the setup default and can be disabled explicitly. Its startup/readiness does not gate business services. Existing diagnostic rows are not automatically deleted; retention is still an operator decision. Backups preserve diagnostic configuration and fresh-store startup capability, while raw historical ClickHouse rows remain excluded and disclosed.

Verify correlated requests and audit, hostile-input redaction, ingest read denial, bounded outage behavior and continued business mutations, restart/recovery of ingestion, network exposure, and idle/resource footprint. The code uses ClickHouse's documented HTTP INSERT with JSONEachRow and grants: [HTTP interface](https://clickhouse.com/docs/interfaces/http), [GRANT](https://clickhouse.com/docs/sql-reference/statements/grant).

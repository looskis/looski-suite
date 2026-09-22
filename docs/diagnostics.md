# Local operational diagnostics

`LOOSKI_DIAGNOSTICS_MODE=local` enables the pinned ClickHouse service and asynchronous API/worker diagnostic pipeline. `disabled` stops collection; PostgreSQL transactional audit continues independently. Setup stores a separate, installation-derived ingestion credential in a private file. ClickHouse publishes no host ports, and the API/worker account can only insert into the diagnostic table.

Events contain generated request/event IDs, time, service, a route template or bounded worker outcome, HTTP status and elapsed time. A response's `X-Request-Id` correlates the diagnostic request with PostgreSQL audit. There are no raw paths, query strings, request/response bodies, headers, tokens, uploaded bytes or arbitrary error text in the diagnostic schema.

Each process has a 1,024-event memory queue, 64-event batches, one-second flush and a two-second send timeout. Full queues and failed batches drop events; aggregate warnings report loss. No request waits for a ClickHouse write, and ClickHouse failure does not fail application readiness or business transactions. These lossy events are operational evidence, not an audit substitute.

There is no automatic diagnostic deletion or disk spool. Monitor the diagnostic volume and choose a retention policy before substantial production traffic. Raw ClickHouse history is not in the encrypted PostgreSQL recovery archive; recovered installations start with diagnostics disabled and can later initialize a fresh log store. Diagnostic SQL access is an installation administration capability, not a CRM workspace permission.

Tests cover fixed-field redaction, correlation with audit, rejection of reads using the ingestion credential, twelve successful mutations during a ClickHouse outage, resumed ingestion after restart, and absence of host ports. The measured small fixture used approximately 3.5 KiB across three data parts; that is not a forecast for production traffic. The measured ClickHouse process used about 677 MiB during test activity, substantially more than API/web/worker combined. See [verification and footprint](implementation/diagnostics-slice.md) for conditions and limits.

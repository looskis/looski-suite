CREATE DATABASE IF NOT EXISTS looski_diagnostics;
CREATE TABLE IF NOT EXISTS looski_diagnostics.events
(
 timestamp_ms UInt64,
 event_id UUID,
 request_id Nullable(UUID),
 service LowCardinality(String),
 kind LowCardinality(String),
 operation LowCardinality(String),
 status UInt16,
 duration_ms UInt64
)
ENGINE = MergeTree ORDER BY (service, timestamp_ms, event_id);

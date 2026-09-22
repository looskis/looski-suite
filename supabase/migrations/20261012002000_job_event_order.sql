-- Transaction start timestamps do not establish job transition order.
ALTER TABLE suite.job_events ADD COLUMN event_sequence bigint GENERATED ALWAYS AS IDENTITY;
CREATE INDEX ON suite.job_events(workspace_id,job_id,event_sequence);
GRANT USAGE,SELECT ON SEQUENCE suite.job_events_event_sequence_seq TO looski_runtime;
UPDATE suite.schema_version SET version=21;

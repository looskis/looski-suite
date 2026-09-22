-- Existing audit entries retain unknown attribution rather than invented history.
ALTER TABLE suite.audit_events ADD COLUMN source_interface text NOT NULL DEFAULT 'unknown'
    CHECK(source_interface IN ('unknown','operator','api','web','cli','mcp'));
UPDATE suite.schema_version SET version=5;

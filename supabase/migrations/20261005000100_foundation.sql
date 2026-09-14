-- SPDX-License-Identifier: AGPL-3.0-only
-- Auth schema and its migration history remain owned by Supabase Auth.
CREATE EXTENSION IF NOT EXISTS pgmq;

CREATE SCHEMA suite;
REVOKE ALL ON SCHEMA suite FROM PUBLIC, anon, authenticated;
GRANT USAGE ON SCHEMA suite TO looski_runtime;

CREATE TABLE suite.schema_version (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    version integer NOT NULL CHECK (version > 0)
);
INSERT INTO suite.schema_version (version) VALUES (1);
GRANT SELECT ON suite.schema_version TO looski_runtime;

-- No business tables/grants until verified identity and authorization exist.

ALTER TABLE suite.audit_events DROP CONSTRAINT audit_events_source_interface_check,
 ADD CONSTRAINT audit_events_source_interface_check CHECK(source_interface IN ('unknown','operator','api','web','cli','mcp','worker'));
ALTER TABLE crm.deal_status_changes ADD COLUMN transition_sequence bigint GENERATED ALWAYS AS IDENTITY;
CREATE INDEX ON crm.deal_status_changes(workspace_id,deal_id,transition_sequence DESC);
GRANT USAGE,SELECT ON SEQUENCE crm.deal_status_changes_transition_sequence_seq TO looski_runtime;
-- Durable job state owns retries; logged PGMQ transports claim notifications.
SELECT pgmq.create('looski_jobs');
GRANT USAGE ON SCHEMA pgmq TO looski_runtime;
GRANT SELECT,INSERT,UPDATE,DELETE ON pgmq.q_looski_jobs TO looski_runtime;
GRANT USAGE,SELECT ON SEQUENCE pgmq.q_looski_jobs_msg_id_seq TO looski_runtime;
CREATE TABLE suite.jobs (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 kind text NOT NULL CHECK(kind='crm.coordination'),deduplication_key text NOT NULL CHECK(length(deduplication_key) BETWEEN 1 AND 200),
 actor_user_id uuid NOT NULL REFERENCES suite.users(id),actor_membership_id uuid NOT NULL,
 actor_issuer text NOT NULL,actor_subject text NOT NULL,
 state text NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','completed','blocked','exhausted','cancelled')),
 attempts integer NOT NULL DEFAULT 0 CHECK(attempts BETWEEN 0 AND 5),
 message_id bigint,lease_token uuid,lease_until timestamptz,error_code text,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,kind,deduplication_key),
 FOREIGN KEY(workspace_id,actor_membership_id) REFERENCES suite.memberships(workspace_id,id),
 CHECK((state='running')=(lease_token IS NOT NULL AND lease_until IS NOT NULL)),
 CHECK(error_code IS NULL OR error_code IN ('authority','unavailable','invalid_input','database','lease_expired','module_disabled'))
);
CREATE INDEX ON suite.jobs(workspace_id,created_at,id);
CREATE TABLE suite.job_events (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,job_id uuid NOT NULL,
 state text NOT NULL CHECK(state IN ('queued','running','completed','blocked','exhausted','cancelled')),
 attempt integer NOT NULL CHECK(attempt BETWEEN 0 AND 5),error_code text,created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,job_id) REFERENCES suite.jobs(workspace_id,id)
);
CREATE INDEX ON suite.job_events(workspace_id,job_id,created_at,id);
CREATE TABLE crm.coordination_rules (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 status_id uuid NOT NULL,revision_id uuid NOT NULL,archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,status_id) REFERENCES crm.deal_statuses(workspace_id,id),
 FOREIGN KEY(workspace_id,revision_id) REFERENCES crm.coordination_revisions(workspace_id,id)
);
CREATE UNIQUE INDEX ON crm.coordination_rules(workspace_id,status_id) WHERE archived_at IS NULL;
CREATE TABLE crm.coordination_runs (
 workspace_id uuid NOT NULL,job_id uuid PRIMARY KEY,rule_id uuid NOT NULL,deal_id uuid NOT NULL,status_id uuid NOT NULL,revision_id uuid NOT NULL,transition_audit_id uuid NOT NULL,base_on date NOT NULL,instance_id uuid,
 UNIQUE(workspace_id,rule_id,transition_audit_id),
 FOREIGN KEY(workspace_id,job_id) REFERENCES suite.jobs(workspace_id,id),
 FOREIGN KEY(workspace_id,rule_id) REFERENCES crm.coordination_rules(workspace_id,id),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,status_id) REFERENCES crm.deal_statuses(workspace_id,id),
 FOREIGN KEY(workspace_id,revision_id) REFERENCES crm.coordination_revisions(workspace_id,id),
 FOREIGN KEY(workspace_id,transition_audit_id) REFERENCES suite.audit_events(workspace_id,id),
 FOREIGN KEY(workspace_id,instance_id) REFERENCES crm.deal_coordination_instances(workspace_id,id)
);
GRANT SELECT,INSERT,UPDATE ON suite.jobs,crm.coordination_rules,crm.coordination_runs TO looski_runtime;
GRANT SELECT,INSERT ON suite.job_events TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES('workspace.jobs.read','Inspect safe job state and attempt history'),('workspace.jobs.manage','Explicitly resume or cancel durable jobs');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p WHERE r.key='owner' AND p.key IN ('workspace.jobs.read','workspace.jobs.manage');
UPDATE suite.schema_version SET version=20;

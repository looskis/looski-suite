-- Coordination definitions pin task specifications; instances retain their work.
CREATE TABLE crm.coordination_templates (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),current_revision bigint NOT NULL DEFAULT 1 CHECK(current_revision>0),
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),UNIQUE(workspace_id,id)
);
CREATE INDEX ON crm.coordination_templates(workspace_id,created_at,id);
CREATE TABLE crm.coordination_revisions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,template_id uuid NOT NULL,
 number bigint NOT NULL CHECK(number>0),name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 audit_event_id uuid NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,template_id,number),
 FOREIGN KEY(workspace_id,template_id) REFERENCES crm.coordination_templates(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
ALTER TABLE crm.coordination_templates ADD FOREIGN KEY(workspace_id,id,current_revision) REFERENCES crm.coordination_revisions(workspace_id,template_id,number) DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE crm.coordination_steps (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,revision_id uuid NOT NULL,
 position integer NOT NULL CHECK(position BETWEEN 0 AND 24),label text NOT NULL CHECK(length(btrim(label)) BETWEEN 1 AND 200),
 task_template_revision_id uuid NOT NULL,
 UNIQUE(workspace_id,revision_id,position),UNIQUE(workspace_id,revision_id,id),
 FOREIGN KEY(workspace_id,revision_id) REFERENCES crm.coordination_revisions(workspace_id,id),
 FOREIGN KEY(workspace_id,task_template_revision_id) REFERENCES tasks.template_revisions(workspace_id,id)
);
CREATE TABLE crm.deal_coordination_instances (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,deal_id uuid NOT NULL,revision_id uuid NOT NULL,
 base_on date NOT NULL,created_by_membership_id uuid NOT NULL,audit_event_id uuid NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,id,deal_id,revision_id),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,revision_id) REFERENCES crm.coordination_revisions(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.deal_coordination_instances(workspace_id,deal_id,created_at,id);
CREATE TABLE crm.deal_coordination_items (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,instance_id uuid NOT NULL,deal_id uuid NOT NULL,revision_id uuid NOT NULL,step_id uuid NOT NULL,task_id uuid NOT NULL,
 UNIQUE(workspace_id,instance_id,step_id),UNIQUE(workspace_id,task_id),
 FOREIGN KEY(workspace_id,instance_id,deal_id,revision_id) REFERENCES crm.deal_coordination_instances(workspace_id,id,deal_id,revision_id),
 FOREIGN KEY(workspace_id,revision_id,step_id) REFERENCES crm.coordination_steps(workspace_id,revision_id,id),
 FOREIGN KEY(workspace_id,deal_id,task_id) REFERENCES crm.deal_tasks(workspace_id,deal_id,task_id)
);
GRANT SELECT,INSERT,UPDATE ON crm.coordination_templates TO looski_runtime;
GRANT SELECT,INSERT ON crm.coordination_revisions,crm.coordination_steps,crm.deal_coordination_instances,crm.deal_coordination_items TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES('crm.deals.coordination.apply','Apply permitted coordination revisions to deals');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) SELECT workspace_id,id,'crm.deals.coordination.apply' FROM suite.roles WHERE key='owner';
UPDATE suite.schema_version SET version=19;
-- A bounded 25-item application may return full task snapshots. Requests remain 64 KiB.
ALTER TABLE suite.idempotency_records DROP CONSTRAINT idempotency_records_result_check,
 ADD CONSTRAINT idempotency_records_result_check CHECK(octet_length(result::text)<=2097152);

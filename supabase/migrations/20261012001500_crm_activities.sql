-- Configurable activity behavior; every activity has one real, scoped anchor.
CREATE TABLE crm.activity_types (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 behavior text NOT NULL CHECK(behavior IN ('note','interaction')),
 position integer NOT NULL DEFAULT 0, archived_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id), UNIQUE(workspace_id,id,behavior)
);
CREATE INDEX ON crm.activity_types(workspace_id,created_at,id);
CREATE TABLE crm.activities (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 contact_id uuid, opportunity_id uuid, deal_id uuid,
 type_id uuid NOT NULL, type_name text NOT NULL CHECK(length(type_name) BETWEEN 1 AND 120),
 behavior text NOT NULL CHECK(behavior IN ('note','interaction')),
 subject text NOT NULL CHECK(length(btrim(subject)) BETWEEN 1 AND 200),
 body text NOT NULL DEFAULT '' CHECK(length(body)<=10000),
 occurred_at timestamptz NOT NULL CHECK(isfinite(occurred_at)),
 duration_minutes integer CHECK(duration_minutes BETWEEN 0 AND 10080),
 responsible_membership_id uuid NOT NULL, archived_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 CHECK(num_nonnulls(contact_id,opportunity_id,deal_id)=1),
 CHECK(behavior='interaction' OR duration_minutes IS NULL),
 FOREIGN KEY(workspace_id,type_id,behavior) REFERENCES crm.activity_types(workspace_id,id,behavior),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,opportunity_id) REFERENCES crm.opportunities(workspace_id,id),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,responsible_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON crm.activities(workspace_id,contact_id,created_at,id) WHERE contact_id IS NOT NULL;
CREATE INDEX ON crm.activities(workspace_id,opportunity_id,created_at,id) WHERE opportunity_id IS NOT NULL;
CREATE INDEX ON crm.activities(workspace_id,deal_id,created_at,id) WHERE deal_id IS NOT NULL;
CREATE INDEX ON crm.activities(workspace_id,responsible_membership_id,created_at,id);
CREATE TABLE crm.activity_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL, activity_id uuid NOT NULL,
 snapshot jsonb NOT NULL CHECK(jsonb_typeof(snapshot)='object' AND pg_column_size(snapshot)<=100000),
 audit_event_id uuid NOT NULL, occurred_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,activity_id) REFERENCES crm.activities(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.activity_changes(workspace_id,activity_id,occurred_at,id);
CREATE INDEX ON crm.activity_changes(workspace_id,audit_event_id);
GRANT SELECT,INSERT,UPDATE ON crm.activity_types,crm.activities TO looski_runtime;
GRANT SELECT,INSERT ON crm.activity_changes TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('crm.activities.read','Read permitted CRM activities and history'),
 ('crm.activities.create','Record CRM activities with permitted responsibility'),
 ('crm.activities.update','Edit permitted CRM activities'),
 ('crm.activities.archive','Archive or restore permitted CRM activities');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key LIKE 'crm.activities.%';
UPDATE suite.schema_version SET version=16;

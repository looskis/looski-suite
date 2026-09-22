-- Shared work specifications are independent of optional CRM.
CREATE TABLE tasks.task_types (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120), position integer NOT NULL DEFAULT 0,
 archived_at timestamptz, version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(), UNIQUE(workspace_id,id)
);
CREATE INDEX ON tasks.task_types(workspace_id,(archived_at IS NOT NULL),position,created_at,id);
ALTER TABLE tasks.tasks ADD COLUMN type_id uuid,ADD COLUMN type_name text,
 ADD FOREIGN KEY(workspace_id,type_id) REFERENCES tasks.task_types(workspace_id,id),
 ADD CHECK((type_id IS NULL)=(type_name IS NULL));
CREATE TABLE tasks.templates (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 current_revision bigint NOT NULL DEFAULT 1 CHECK(current_revision>0),
 archived_at timestamptz, version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(), UNIQUE(workspace_id,id)
);
CREATE INDEX ON tasks.templates(workspace_id,created_at,id);
CREATE TABLE tasks.template_revisions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL,template_id uuid NOT NULL,
 number bigint NOT NULL CHECK(number>0),
 title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 200),notes text NOT NULL CHECK(length(notes)<=10000),
 type_id uuid,type_name text,
 assignment text NOT NULL CHECK(assignment IN ('initiator','context_owner','fixed')),
 fixed_membership_id uuid,due_offset_days integer NOT NULL CHECK(due_offset_days BETWEEN -3650 AND 3650),
 audit_event_id uuid NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,template_id,number),
 FOREIGN KEY(workspace_id,template_id) REFERENCES tasks.templates(workspace_id,id),
 FOREIGN KEY(workspace_id,type_id) REFERENCES tasks.task_types(workspace_id,id),
 FOREIGN KEY(workspace_id,fixed_membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id),
 CHECK((assignment='fixed')=(fixed_membership_id IS NOT NULL)),CHECK((type_id IS NULL)=(type_name IS NULL))
);
ALTER TABLE tasks.tasks ADD COLUMN source_template_revision_id uuid,
 ADD FOREIGN KEY(workspace_id,source_template_revision_id) REFERENCES tasks.template_revisions(workspace_id,id);
ALTER TABLE tasks.templates ADD FOREIGN KEY(workspace_id,id,current_revision) REFERENCES tasks.template_revisions(workspace_id,template_id,number) DEFERRABLE INITIALLY DEFERRED;
CREATE INDEX ON tasks.template_revisions(workspace_id,template_id,number,id);
CREATE INDEX ON tasks.template_revisions(workspace_id,audit_event_id);
GRANT SELECT,INSERT,UPDATE ON tasks.task_types,tasks.templates TO looski_runtime;
GRANT SELECT,INSERT ON tasks.template_revisions TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('tasks.configuration.read','Read shared task types and template revisions'),
 ('tasks.configuration.manage','Configure shared task types and reusable templates');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('tasks.configuration.read','tasks.configuration.manage');
UPDATE suite.schema_version SET version=18;

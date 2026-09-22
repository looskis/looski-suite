-- Shared task lifecycle is independent of optional applications.
CREATE SCHEMA tasks;
GRANT USAGE ON SCHEMA tasks TO looski_runtime;
CREATE TABLE tasks.tasks (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 200),
 notes text NOT NULL DEFAULT '' CHECK(length(notes)<=10000),
 assigned_membership_id uuid NOT NULL,
 due_on date, status text NOT NULL DEFAULT 'open' CHECK(status IN ('open','in_progress','completed','cancelled')),
 completed_at timestamptz, archived_at timestamptz,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,assigned_membership_id) REFERENCES suite.memberships(workspace_id,id),
 CHECK((status='completed')=(completed_at IS NOT NULL))
);
CREATE INDEX ON tasks.tasks(workspace_id,created_at,id);
CREATE INDEX ON tasks.tasks(workspace_id,assigned_membership_id,created_at,id);
CREATE INDEX ON tasks.tasks(workspace_id,status,due_on,id);
CREATE TABLE tasks.task_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL, task_id uuid NOT NULL,
 snapshot jsonb NOT NULL CHECK(jsonb_typeof(snapshot)='object' AND pg_column_size(snapshot)<=100000),
 audit_event_id uuid NOT NULL, occurred_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,task_id) REFERENCES tasks.tasks(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON tasks.task_changes(workspace_id,task_id,occurred_at,id);
CREATE INDEX ON tasks.task_changes(workspace_id,audit_event_id);
GRANT SELECT,INSERT,UPDATE ON tasks.tasks TO looski_runtime;
GRANT SELECT,INSERT ON tasks.task_changes TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('tasks.read','Read permitted tasks and task history'),('tasks.create','Create tasks with permitted assignment'),
 ('tasks.update','Edit permitted tasks'),('tasks.archive','Archive or restore permitted tasks');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('tasks.read','tasks.create','tasks.update','tasks.archive');
UPDATE suite.schema_version SET version=14;

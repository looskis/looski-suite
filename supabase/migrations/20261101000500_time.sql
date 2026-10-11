-- Time remains useful with every other application disabled.
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','forms','cms','projects','knowledge','helpdesk','time'));
CREATE SCHEMA time_tracking;
REVOKE ALL ON SCHEMA time_tracking FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA time_tracking TO looski_runtime;
CREATE TABLE time_tracking.entries (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 membership_id uuid NOT NULL, project_id uuid,
 description text NOT NULL CHECK(length(btrim(description)) BETWEEN 1 AND 2000),
 work_on date NOT NULL, billable boolean NOT NULL, archived boolean NOT NULL DEFAULT false,
 state text NOT NULL CHECK(state IN ('running','paused','stopped')),
 duration_ms bigint NOT NULL CHECK(duration_ms>=0), running_since timestamptz,
 version bigint NOT NULL DEFAULT 1, created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK((state='running')=(running_since IS NOT NULL)), CHECK(NOT archived OR state='stopped'),
 UNIQUE(workspace_id,id), FOREIGN KEY(workspace_id,membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,project_id) REFERENCES projects.projects(workspace_id,id)
);
CREATE UNIQUE INDEX one_running_timer ON time_tracking.entries(workspace_id,membership_id) WHERE state='running';
CREATE INDEX ON time_tracking.entries(workspace_id,archived,work_on,id);
CREATE TABLE time_tracking.budgets (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120), project_id uuid,
 from_on date NOT NULL, through_on date NOT NULL CHECK(through_on>=from_on),
 budget_ms bigint NOT NULL CHECK(budget_ms>0), billable_only boolean NOT NULL,
 archived boolean NOT NULL DEFAULT false, version bigint NOT NULL DEFAULT 1,
 FOREIGN KEY(workspace_id,project_id) REFERENCES projects.projects(workspace_id,id)
);
GRANT SELECT,INSERT,UPDATE ON time_tracking.entries,time_tracking.budgets TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('time.read','Read entries and totals; assigned scope is own entries'),
 ('time.manage','Create own time and edit entries; assigned scope is own entries'),
 ('time.budgets.manage','Manage workspace time budgets; requires workspace scope');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p WHERE r.key='owner' AND p.key LIKE 'time.%';
UPDATE suite.schema_version SET version=35;

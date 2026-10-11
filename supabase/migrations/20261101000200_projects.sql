-- Optional Projects: grouping, membership, boards and comments over shared Tasks,
-- plus the explicitly activated deal-to-project recipe. Task state stays in tasks.*;
-- nothing here is granted write access to that schema.
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','forms','cms','projects'));
CREATE SCHEMA projects;
REVOKE ALL ON SCHEMA projects FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA projects TO looski_runtime;
CREATE TABLE projects.projects (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 description text NOT NULL DEFAULT '' CHECK(length(description)<=4000),
 status text NOT NULL DEFAULT 'active' CHECK(status IN ('active','on_hold','completed','cancelled')),
 due_on date,created_by_membership_id uuid NOT NULL,archived_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON projects.projects(workspace_id,created_at,id);
-- Membership is the record scope of an `assigned` Projects grant; it grants nothing alone.
CREATE TABLE projects.members (
 workspace_id uuid NOT NULL,project_id uuid NOT NULL,membership_id uuid NOT NULL,
 added_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,project_id,membership_id),
 FOREIGN KEY(workspace_id,project_id) REFERENCES projects.projects(workspace_id,id),
 FOREIGN KEY(workspace_id,membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON projects.members(workspace_id,membership_id,project_id);
CREATE TABLE projects.columns (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,project_id uuid NOT NULL,
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 60),
 position integer NOT NULL CHECK(position>=0),
 -- Moving a card into this column asks Tasks for this status; NULL changes nothing.
 task_status text CHECK(task_status IN ('open','in_progress','completed','cancelled')),
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,project_id,id),
 UNIQUE(workspace_id,project_id,position) DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(workspace_id,project_id) REFERENCES projects.projects(workspace_id,id)
);
-- A typed link to one shared task, never a copy of its state. One row per project and
-- task, so comments survive removal and return when the task is added again.
CREATE TABLE projects.cards (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,project_id uuid NOT NULL,
 task_id uuid NOT NULL,column_id uuid,position integer CHECK(position>=0),removed_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,project_id,id),UNIQUE(workspace_id,project_id,task_id),
 -- No two cards share a place in a column, whatever the interleaving of moves.
 UNIQUE(workspace_id,column_id,position) DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(workspace_id,project_id) REFERENCES projects.projects(workspace_id,id),
 FOREIGN KEY(workspace_id,task_id) REFERENCES tasks.tasks(workspace_id,id),
 FOREIGN KEY(workspace_id,project_id,column_id) REFERENCES projects.columns(workspace_id,project_id,id),
 CHECK((removed_at IS NULL)=(column_id IS NOT NULL) AND (column_id IS NULL)=(position IS NULL))
);
CREATE UNIQUE INDEX cards_one_active_project ON projects.cards(workspace_id,task_id) WHERE removed_at IS NULL;
CREATE INDEX ON projects.cards(workspace_id,project_id,created_at,id);
-- Bodies are plain source; only the typed blocks parsed from it reach a renderer.
CREATE TABLE projects.comments (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,project_id uuid NOT NULL,card_id uuid NOT NULL,
 author_membership_id uuid NOT NULL,body text NOT NULL CHECK(length(body)<=10000),
 edited_at timestamptz,removed_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,project_id,card_id) REFERENCES projects.cards(workspace_id,project_id,id),
 FOREIGN KEY(workspace_id,author_membership_id) REFERENCES suite.memberships(workspace_id,id),
 CHECK(CASE WHEN removed_at IS NULL THEN length(btrim(body))>=1 ELSE body='' END)
);
CREATE INDEX ON projects.comments(workspace_id,card_id,created_at,id);
CREATE INDEX ON projects.comments(workspace_id,project_id,created_at,id);
CREATE TABLE projects.deal_recipes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 revision_id uuid,status_id uuid,active boolean NOT NULL DEFAULT false,
 -- Authority recorded at activation; every run re-checks its current permissions.
 actor_user_id uuid,actor_membership_id uuid,actor_issuer text,actor_subject text,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id) REFERENCES suite.workspaces(id),
 FOREIGN KEY(workspace_id,status_id) REFERENCES crm.deal_statuses(workspace_id,id),
 FOREIGN KEY(actor_user_id) REFERENCES suite.users(id),
 FOREIGN KEY(workspace_id,actor_membership_id) REFERENCES suite.memberships(workspace_id,id),
 CHECK(NOT active OR (revision_id IS NOT NULL AND status_id IS NOT NULL AND actor_membership_id IS NOT NULL AND actor_issuer IS NOT NULL AND actor_subject IS NOT NULL))
);
CREATE UNIQUE INDEX deal_recipes_one_active_per_status ON projects.deal_recipes(workspace_id,status_id) WHERE active;
CREATE INDEX ON projects.deal_recipes(workspace_id,created_at,id);
CREATE TABLE projects.deal_recipe_revisions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,recipe_id uuid NOT NULL,
 number integer NOT NULL CHECK(number>0),configuration jsonb NOT NULL CHECK(octet_length(configuration::text)<=16384),
 created_by_membership_id uuid NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,recipe_id,id),UNIQUE(workspace_id,recipe_id,number),
 FOREIGN KEY(workspace_id,recipe_id) REFERENCES projects.deal_recipes(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
ALTER TABLE projects.deal_recipes ADD FOREIGN KEY(workspace_id,id,revision_id) REFERENCES projects.deal_recipe_revisions(workspace_id,recipe_id,id);
CREATE TABLE projects.deal_conversions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,recipe_id uuid NOT NULL,revision_id uuid NOT NULL,
 deal_id uuid NOT NULL,job_id uuid NOT NULL,
 state text NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','completed')),project_id uuid,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,job_id),
 -- One configured conversion per deal and recipe, whatever the retries or re-entries.
 UNIQUE(workspace_id,recipe_id,deal_id),UNIQUE(workspace_id,project_id),
 FOREIGN KEY(workspace_id,recipe_id,revision_id) REFERENCES projects.deal_recipe_revisions(workspace_id,recipe_id,id),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,job_id) REFERENCES suite.jobs(workspace_id,id),
 FOREIGN KEY(workspace_id,project_id) REFERENCES projects.projects(workspace_id,id),
 CHECK((state='completed')=(project_id IS NOT NULL))
);
CREATE INDEX ON projects.deal_conversions(workspace_id,recipe_id,created_at,id);
ALTER TABLE suite.jobs DROP CONSTRAINT jobs_kind_check,
 ADD CONSTRAINT jobs_kind_check CHECK(kind IN ('crm.coordination','workspace.invitation_email','forms.crm_inquiry','mail.notification','projects.deal_conversion'));
GRANT SELECT,INSERT ON projects.projects,projects.cards,projects.comments,projects.deal_recipes,projects.deal_recipe_revisions,projects.deal_conversions TO looski_runtime;
GRANT SELECT,INSERT,DELETE ON projects.members,projects.columns TO looski_runtime;
GRANT UPDATE(name,description,status,due_on,archived_at,version,updated_at) ON projects.projects TO looski_runtime;
GRANT UPDATE(name,position,task_status,version) ON projects.columns TO looski_runtime;
GRANT UPDATE(column_id,position,removed_at,version,updated_at) ON projects.cards TO looski_runtime;
GRANT UPDATE(body,edited_at,removed_at,version,updated_at) ON projects.comments TO looski_runtime;
GRANT UPDATE(name,revision_id,status_id,active,actor_user_id,actor_membership_id,actor_issuer,actor_subject,version,updated_at) ON projects.deal_recipes TO looski_runtime;
GRANT UPDATE(state,project_id,version,updated_at) ON projects.deal_conversions TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('projects.read','Read projects, members, boards and delivery views; assigned scope means projects the member belongs to'),
 ('projects.create','Create projects; the creator becomes a member'),
 ('projects.manage','Edit, archive and restore projects and their board columns'),
 ('projects.members.manage','Add and remove project members'),
 ('projects.board.edit','Add, remove, move and schedule tasks on a project board'),
 ('projects.comments.write','Comment on tasks in a project and edit one''s own comments'),
 ('projects.comments.moderate','Remove other members'' comments in a project'),
 ('projects.recipes.manage','Configure, activate and retry the deal-to-project recipe');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('projects.read','projects.create','projects.manage','projects.members.manage','projects.board.edit','projects.comments.write','projects.comments.moderate','projects.recipes.manage');
UPDATE suite.schema_version SET version=32;

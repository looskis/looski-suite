-- Optional Forms: editable drafts, immutable published versions, public
-- submissions and the explicitly activated Forms-to-CRM inquiry recipe.
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','forms'));
CREATE SCHEMA forms;
REVOKE ALL ON SCHEMA forms FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA forms TO looski_runtime;
CREATE TABLE forms.forms (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 description text NOT NULL DEFAULT '' CHECK(length(description)<=2000),
 confirmation_message text NOT NULL DEFAULT '' CHECK(length(confirmation_message)<=2000),
 draft jsonb NOT NULL CHECK(octet_length(draft::text)<=262144),
 -- Installation-unique bearer of public submission rights for this form only.
 public_key text NOT NULL UNIQUE CHECK(public_key ~ '^[A-Za-z0-9_-]{22}$'),
 published_version_id uuid,accepting boolean NOT NULL DEFAULT false,
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 CHECK(NOT accepting OR published_version_id IS NOT NULL)
);
CREATE INDEX ON forms.forms(workspace_id,created_at,id);
CREATE TABLE forms.versions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,form_id uuid NOT NULL,
 number integer NOT NULL CHECK(number>0),name text NOT NULL,definition jsonb NOT NULL CHECK(octet_length(definition::text)<=262144),
 published_by_membership_id uuid NOT NULL,published_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,form_id,id),UNIQUE(workspace_id,form_id,number),
 FOREIGN KEY(workspace_id,form_id) REFERENCES forms.forms(workspace_id,id),
 FOREIGN KEY(workspace_id,published_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
ALTER TABLE forms.forms ADD FOREIGN KEY(workspace_id,id,published_version_id) REFERENCES forms.versions(workspace_id,form_id,id);
CREATE TABLE forms.notification_recipients (
 workspace_id uuid NOT NULL,form_id uuid NOT NULL,membership_id uuid NOT NULL,
 PRIMARY KEY(workspace_id,form_id,membership_id),
 FOREIGN KEY(workspace_id,form_id) REFERENCES forms.forms(workspace_id,id),
 FOREIGN KEY(workspace_id,membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE TABLE forms.submissions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,form_id uuid NOT NULL,version_id uuid NOT NULL,
 -- Original visible answers keyed by stable field keys; never rewritten.
 answers jsonb NOT NULL CHECK(jsonb_typeof(answers)='object' AND octet_length(answers::text)<=131072),
 source text NOT NULL CHECK(source IN ('public')),
 submission_key text NOT NULL CHECK(submission_key ~ '^[!-~]{16,128}$'),
 submitted_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,form_id,submission_key),
 FOREIGN KEY(workspace_id,form_id,version_id) REFERENCES forms.versions(workspace_id,form_id,id)
);
CREATE INDEX ON forms.submissions(workspace_id,form_id,submitted_at,id);
CREATE TABLE forms.crm_recipes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,form_id uuid NOT NULL,
 revision_id uuid,active boolean NOT NULL DEFAULT false,
 -- Authority recorded at activation; every run re-checks its current permissions.
 actor_user_id uuid,actor_membership_id uuid,actor_issuer text,actor_subject text,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,form_id),
 FOREIGN KEY(workspace_id,form_id) REFERENCES forms.forms(workspace_id,id),
 FOREIGN KEY(actor_user_id) REFERENCES suite.users(id),
 FOREIGN KEY(workspace_id,actor_membership_id) REFERENCES suite.memberships(workspace_id,id),
 CHECK(NOT active OR (revision_id IS NOT NULL AND actor_membership_id IS NOT NULL AND actor_issuer IS NOT NULL AND actor_subject IS NOT NULL))
);
CREATE TABLE forms.crm_recipe_revisions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,recipe_id uuid NOT NULL,
 number integer NOT NULL CHECK(number>0),mapping jsonb NOT NULL CHECK(octet_length(mapping::text)<=16384),
 created_by_membership_id uuid NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,recipe_id,id),UNIQUE(workspace_id,recipe_id,number),
 FOREIGN KEY(workspace_id,recipe_id) REFERENCES forms.crm_recipes(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
ALTER TABLE forms.crm_recipes ADD FOREIGN KEY(workspace_id,id,revision_id) REFERENCES forms.crm_recipe_revisions(workspace_id,recipe_id,id);
CREATE TABLE forms.crm_recipe_runs (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,recipe_id uuid NOT NULL,revision_id uuid NOT NULL,
 submission_id uuid NOT NULL,job_id uuid NOT NULL,
 state text NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','awaiting_review','completed','dismissed')),
 intake_id uuid,contact_id uuid,opportunity_id uuid,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,job_id),
 -- One configured run per submission and recipe, whatever the retries.
 UNIQUE(workspace_id,recipe_id,submission_id),
 FOREIGN KEY(workspace_id,recipe_id,revision_id) REFERENCES forms.crm_recipe_revisions(workspace_id,recipe_id,id),
 FOREIGN KEY(workspace_id,submission_id) REFERENCES forms.submissions(workspace_id,id),
 FOREIGN KEY(workspace_id,job_id) REFERENCES suite.jobs(workspace_id,id),
 FOREIGN KEY(workspace_id,intake_id) REFERENCES contacts.intakes(workspace_id,id),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,opportunity_id) REFERENCES crm.opportunities(workspace_id,id),
 CHECK((state='completed')=(opportunity_id IS NOT NULL))
);
CREATE INDEX ON forms.crm_recipe_runs(workspace_id,recipe_id,created_at,id);
CREATE INDEX ON forms.crm_recipe_runs(workspace_id,submission_id);
-- Outbound mail is delivered at least once by the worker through SMTP.
CREATE TABLE suite.outbound_messages (
 job_id uuid PRIMARY KEY,workspace_id uuid NOT NULL,recipient_membership_id uuid NOT NULL,
 purpose text NOT NULL CHECK(purpose IN ('forms.submission')),
 subject text NOT NULL CHECK(length(subject) BETWEEN 1 AND 200),body text NOT NULL CHECK(length(body) BETWEEN 1 AND 4000),
 reference_id uuid NOT NULL,link_path text NOT NULL CHECK(length(link_path)<=300 AND link_path ~ '^/[A-Za-z0-9/_-]+$'),
 sent_at timestamptz,created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,job_id) REFERENCES suite.jobs(workspace_id,id),
 FOREIGN KEY(workspace_id,recipient_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
ALTER TABLE suite.jobs DROP CONSTRAINT jobs_kind_check,
 ADD CONSTRAINT jobs_kind_check CHECK(kind IN ('crm.coordination','workspace.invitation_email','forms.crm_inquiry','mail.notification'));
GRANT SELECT,INSERT ON forms.forms,forms.versions,forms.submissions,forms.crm_recipes,forms.crm_recipe_revisions,forms.crm_recipe_runs,suite.outbound_messages TO looski_runtime;
GRANT SELECT,INSERT,DELETE ON forms.notification_recipients TO looski_runtime;
GRANT UPDATE(name,description,confirmation_message,draft,published_version_id,accepting,archived_at,version,updated_at) ON forms.forms TO looski_runtime;
GRANT UPDATE(revision_id,active,actor_user_id,actor_membership_id,actor_issuer,actor_subject,version,updated_at) ON forms.crm_recipes TO looski_runtime;
GRANT UPDATE(job_id,state,intake_id,contact_id,opportunity_id,version,updated_at) ON forms.crm_recipe_runs TO looski_runtime;
GRANT UPDATE(sent_at) ON suite.outbound_messages TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('forms.read','Read form definitions and published versions'),
 ('forms.manage','Create, edit, publish, close and archive forms'),
 ('forms.submissions.read','Read submitted answers and receive submission notifications'),
 ('forms.recipes.manage','Configure, activate and retry the Forms-to-CRM inquiry recipe');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('forms.read','forms.manage','forms.submissions.read','forms.recipes.manage');
UPDATE suite.schema_version SET version=30;

-- Public booking, atomic versioned intake and independently durable follow-up.
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','forms','cms','projects','knowledge','helpdesk','time','scheduling'));
CREATE SCHEMA scheduling;
REVOKE ALL ON SCHEMA scheduling FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA scheduling TO looski_runtime;
CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA extensions;
SET search_path=public,extensions;
CREATE TABLE scheduling.types (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 host_id uuid NOT NULL,public_key uuid NOT NULL DEFAULT gen_random_uuid() UNIQUE,
 settings jsonb NOT NULL,version bigint NOT NULL DEFAULT 1,
 actor_user_id uuid NOT NULL REFERENCES suite.users(id),actor_membership_id uuid NOT NULL,actor_issuer text NOT NULL,actor_subject text NOT NULL,
 UNIQUE(workspace_id,id),FOREIGN KEY(workspace_id,host_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,actor_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE TABLE scheduling.appointments (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,type_id uuid NOT NULL,host_id uuid NOT NULL,
 name text NOT NULL,timezone text NOT NULL,starts_at timestamptz NOT NULL,ends_at timestamptz NOT NULL CHECK(ends_at>starts_at),
 busy tstzrange NOT NULL,attendee_name text NOT NULL,attendee_email text NOT NULL,
 state text NOT NULL DEFAULT 'confirmed' CHECK(state IN ('confirmed','cancelled')),
 key_hash text NOT NULL CHECK(length(key_hash)=64),request_hash text NOT NULL,
 version bigint NOT NULL DEFAULT 1,form_id uuid,submission_id uuid,
 mapping jsonb,intake_id uuid,contact_id uuid,opportunity_id uuid,
 recipe_state text NOT NULL DEFAULT 'none' CHECK(recipe_state IN ('none','queued','awaiting_review','completed','dismissed')),
 recipe_job_id uuid,created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,key_hash),
 FOREIGN KEY(workspace_id,type_id) REFERENCES scheduling.types(workspace_id,id),
 FOREIGN KEY(workspace_id,host_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,submission_id) REFERENCES forms.submissions(workspace_id,id),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,recipe_job_id) REFERENCES suite.jobs(workspace_id,id),
 EXCLUDE USING gist(workspace_id WITH =,host_id WITH =,busy WITH &&) WHERE(state='confirmed')
);
CREATE INDEX ON scheduling.appointments(workspace_id,id);
CREATE TABLE scheduling.events (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,appointment_id uuid NOT NULL,
 action text NOT NULL,version bigint NOT NULL,starts_at timestamptz NOT NULL,ends_at timestamptz NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,appointment_id) REFERENCES scheduling.appointments(workspace_id,id),
 UNIQUE(workspace_id,appointment_id,version)
);
CREATE TABLE scheduling.retries (
 workspace_id uuid NOT NULL,appointment_id uuid NOT NULL,key text NOT NULL,request_hash text NOT NULL,
 PRIMARY KEY(workspace_id,appointment_id,key),FOREIGN KEY(workspace_id,appointment_id) REFERENCES scheduling.appointments(workspace_id,id)
);
CREATE TABLE scheduling.reminders (
 job_id uuid PRIMARY KEY,workspace_id uuid NOT NULL,appointment_id uuid NOT NULL,version bigint NOT NULL,
 sent_at timestamptz,skipped boolean NOT NULL DEFAULT false,
 FOREIGN KEY(workspace_id,job_id) REFERENCES suite.jobs(workspace_id,id),
 FOREIGN KEY(workspace_id,appointment_id) REFERENCES scheduling.appointments(workspace_id,id)
);
GRANT SELECT,INSERT,UPDATE ON scheduling.types,scheduling.appointments,scheduling.reminders TO looski_runtime;
GRANT SELECT,INSERT ON scheduling.events,scheduling.retries TO looski_runtime;
ALTER TABLE suite.jobs ADD COLUMN available_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE suite.jobs DROP CONSTRAINT jobs_kind_check,
 ADD CONSTRAINT jobs_kind_check CHECK(kind IN ('crm.coordination','workspace.invitation_email','forms.crm_inquiry','mail.notification','projects.deal_conversion','scheduling.crm_inquiry','scheduling.reminder'));
INSERT INTO suite.permissions(key,description) VALUES
 ('scheduling.read','Read appointment types and bookings; assigned scope is own host calendar'),
 ('scheduling.manage','Manage own appointment types and bookings; workspace scope reaches all hosts'),
 ('scheduling.recipes.manage','Configure and run booking CRM follow-up with current authority');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p WHERE r.key='owner' AND p.key LIKE 'scheduling.%';
UPDATE suite.schema_version SET version=36;

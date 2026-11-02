-- Communications owns durable provider history; secrets stay in operator-owned files.
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','forms','cms','projects','knowledge','helpdesk','time','scheduling','communications'));
ALTER TABLE suite.jobs DROP CONSTRAINT jobs_kind_check,
 ADD CONSTRAINT jobs_kind_check CHECK(kind IN ('crm.coordination','workspace.invitation_email','forms.crm_inquiry','mail.notification','projects.deal_conversion','scheduling.crm_inquiry','scheduling.reminder','communications.send'));
CREATE SCHEMA communications;
REVOKE ALL ON SCHEMA communications FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA communications TO looski_runtime;
CREATE TABLE communications.accounts (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 connector_key text NOT NULL UNIQUE, provider text NOT NULL, installation_id text NOT NULL, capabilities jsonb NOT NULL, name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 owner_membership_id uuid NOT NULL, shared boolean NOT NULL DEFAULT false, enabled boolean NOT NULL DEFAULT true,
 cursor bigint NOT NULL DEFAULT 0, sync_at timestamptz, sync_error text, version bigint NOT NULL DEFAULT 1,
 UNIQUE(workspace_id,id), FOREIGN KEY(workspace_id,owner_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE TABLE communications.conversations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL, account_id uuid NOT NULL,
 provider_thread_id text NOT NULL, channel text NOT NULL, subject text NOT NULL DEFAULT '',
 contact_id uuid, archived boolean NOT NULL DEFAULT false, version bigint NOT NULL DEFAULT 1,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id), UNIQUE(workspace_id,account_id,id), UNIQUE(account_id,provider_thread_id),
 FOREIGN KEY(workspace_id,account_id) REFERENCES communications.accounts(workspace_id,id),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id)
);
CREATE INDEX ON communications.conversations(workspace_id,contact_id,id);
CREATE TABLE communications.messages (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL, account_id uuid NOT NULL, conversation_id uuid NOT NULL,
 direction text NOT NULL CHECK(direction IN ('inbound','outbound')), body text NOT NULL,
 sender text, provider_message_id text, provider_dispatch_id text, metadata jsonb NOT NULL DEFAULT '{}',
 state text NOT NULL CHECK(state IN ('received','queued','accepted','sent','delivered','read','failed','unknown')),
 status_cursor bigint NOT NULL DEFAULT 0, job_id uuid REFERENCES suite.jobs(id),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(workspace_id,id), UNIQUE(account_id,provider_message_id), UNIQUE(job_id),
 FOREIGN KEY(workspace_id,account_id,conversation_id) REFERENCES communications.conversations(workspace_id,account_id,id)
);
CREATE INDEX ON communications.messages(workspace_id,conversation_id,created_at,id);
CREATE TABLE communications.events (
 account_id uuid NOT NULL REFERENCES communications.accounts(id), cursor bigint NOT NULL CHECK(cursor>0),
 fingerprint text NOT NULL, payload jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(account_id,cursor)
);
GRANT SELECT,INSERT,UPDATE ON communications.accounts,communications.conversations,communications.messages TO looski_runtime;
GRANT SELECT,INSERT ON communications.events TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('communications.read','Read owned accounts and conversations; workspace scope also reads shared accounts'),
 ('communications.send','Send through visible owned accounts; workspace scope also sends through shared accounts'),
 ('communications.manage','Link contacts and archive conversations in visible accounts'),
 ('communications.configure','Connect and configure owned or shared accounts; workspace scope required');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key LIKE 'communications.%';
UPDATE suite.schema_version SET version=37;

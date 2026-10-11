-- Ticket-scoped customer capabilities, separate from staff authority and contact identity.
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','forms','cms','projects','knowledge','helpdesk'));
CREATE SCHEMA helpdesk;
REVOKE ALL ON SCHEMA helpdesk FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA helpdesk TO looski_runtime;
CREATE TABLE helpdesk.settings (
 workspace_id uuid PRIMARY KEY REFERENCES suite.workspaces(id),
 public_key uuid NOT NULL DEFAULT gen_random_uuid() UNIQUE,
 accepting boolean NOT NULL DEFAULT false,
 version bigint NOT NULL DEFAULT 1
);
CREATE TABLE helpdesk.tickets (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 subject text NOT NULL CHECK(length(btrim(subject)) BETWEEN 1 AND 200),
 requester_name text NOT NULL CHECK(length(btrim(requester_name)) BETWEEN 1 AND 120),
 status text NOT NULL DEFAULT 'open' CHECK(status IN ('open','waiting','resolved')),
 assignee_id uuid,contact_id uuid,archived boolean NOT NULL DEFAULT false,
 customer_key_hash text NOT NULL CHECK(length(customer_key_hash)=64),
 creation_hash text NOT NULL CHECK(length(creation_hash)=64),
 customer_revoked boolean NOT NULL DEFAULT false,
 version bigint NOT NULL DEFAULT 1,created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,customer_key_hash),
 FOREIGN KEY(workspace_id,assignee_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id)
);
CREATE INDEX ON helpdesk.tickets(workspace_id,archived,id);
CREATE INDEX ON helpdesk.tickets(workspace_id,contact_id,id);
CREATE TABLE helpdesk.messages (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,ticket_id uuid NOT NULL,
 kind text NOT NULL CHECK(kind IN ('customer','reply','note')),
 body text NOT NULL CHECK(length(btrim(body)) BETWEEN 1 AND 20000),
 author_membership_id uuid,
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),FOREIGN KEY(workspace_id,ticket_id) REFERENCES helpdesk.tickets(workspace_id,id),
 FOREIGN KEY(workspace_id,author_membership_id) REFERENCES suite.memberships(workspace_id,id),
 CHECK((kind='customer')=(author_membership_id IS NULL))
);
CREATE INDEX ON helpdesk.messages(workspace_id,ticket_id,id);
CREATE TABLE helpdesk.article_links (
 workspace_id uuid NOT NULL,ticket_id uuid NOT NULL,document_id uuid NOT NULL,
 PRIMARY KEY(workspace_id,ticket_id,document_id),
 FOREIGN KEY(workspace_id,ticket_id) REFERENCES helpdesk.tickets(workspace_id,id),
 FOREIGN KEY(workspace_id,document_id) REFERENCES knowledge.documents(workspace_id,id)
);
-- Immutable, metadata-only event history for audit and independently scoped timelines.
CREATE TABLE helpdesk.events (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,ticket_id uuid NOT NULL,
 action text NOT NULL,actor_user_id uuid REFERENCES suite.users(id),
 created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,ticket_id) REFERENCES helpdesk.tickets(workspace_id,id)
);
CREATE INDEX ON helpdesk.events(workspace_id,ticket_id,created_at,id);
CREATE TABLE helpdesk.customer_retries (
 workspace_id uuid NOT NULL,ticket_id uuid NOT NULL,retry_key text NOT NULL CHECK(length(retry_key) BETWEEN 1 AND 128),
 request_hash text NOT NULL,message_id uuid NOT NULL,
 PRIMARY KEY(workspace_id,ticket_id,retry_key),
 FOREIGN KEY(workspace_id,ticket_id) REFERENCES helpdesk.tickets(workspace_id,id),
 FOREIGN KEY(workspace_id,message_id) REFERENCES helpdesk.messages(workspace_id,id)
);
GRANT SELECT,INSERT,UPDATE ON helpdesk.settings,helpdesk.tickets TO looski_runtime;
GRANT SELECT,INSERT ON helpdesk.messages,helpdesk.events,helpdesk.customer_retries TO looski_runtime;
GRANT SELECT,INSERT,DELETE ON helpdesk.article_links TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('helpdesk.read','Read tickets and private notes; assigned scope reaches tickets assigned to this member'),
 ('helpdesk.manage','Set ticket status, assignment, associations, archive and revoke customer access'),
 ('helpdesk.reply','Send public replies or append private internal notes'),
 ('helpdesk.configure','Open or close public ticket intake; workspace scope required');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key LIKE 'helpdesk.%';
UPDATE suite.schema_version SET version=34;

-- Scoped Contacts operations. Supabase continues to own auth.*.
ALTER TABLE suite.role_permissions ADD COLUMN record_scope text NOT NULL DEFAULT 'workspace'
    CHECK (record_scope IN ('workspace', 'assigned'));
ALTER TABLE suite.role_permissions DROP CONSTRAINT role_permissions_pkey;
ALTER TABLE suite.role_permissions ADD PRIMARY KEY (workspace_id, role_id, permission_key, record_scope);
INSERT INTO suite.permissions(key, description) VALUES
 ('contacts.read', 'Read permitted contacts'),
 ('contacts.create', 'Create contacts'),
 ('contacts.update', 'Edit permitted contacts'),
 ('contacts.archive', 'Archive and restore permitted contacts'),
 ('contacts.merge', 'Merge confirmed duplicates'),
 ('contacts.identifiers.share', 'Explicitly record a reviewed shared identifier'),
 ('contacts.delete_permanently', 'Permanently delete unreferenced contacts'),
 ('contacts.export', 'Export permitted contacts');
-- Explicit upgrade grants to the existing installation owner, not wildcard permissions.
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('contacts.read','contacts.create','contacts.update',
 'contacts.archive','contacts.merge','contacts.identifiers.share','contacts.delete_permanently','contacts.export');

CREATE SCHEMA contacts;
REVOKE ALL ON SCHEMA contacts FROM PUBLIC, anon, authenticated;
GRANT USAGE ON SCHEMA contacts TO looski_runtime;
CREATE TABLE contacts.contacts (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 display_name text NOT NULL CHECK (length(btrim(display_name)) BETWEEN 1 AND 200),
 notes text NOT NULL DEFAULT '' CHECK (length(notes)<=10000),
 owner_membership_id uuid NOT NULL,
 archived_at timestamptz,
 merged_into_id uuid,
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(),
 version bigint NOT NULL DEFAULT 1 CHECK (version>0),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,owner_membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,merged_into_id) REFERENCES contacts.contacts(workspace_id,id),
 CHECK (merged_into_id IS NULL OR (merged_into_id<>id AND archived_at IS NOT NULL))
);
CREATE INDEX ON contacts.contacts(workspace_id,created_at,id);
CREATE INDEX ON contacts.contacts(workspace_id,owner_membership_id);
CREATE INDEX ON contacts.contacts(workspace_id,merged_into_id);
CREATE TABLE contacts.identifiers (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 kind text NOT NULL CHECK(kind IN ('email','phone')),
 normalized_value text COLLATE "C" NOT NULL CHECK(length(normalized_value) BETWEEN 3 AND 320),
 UNIQUE(workspace_id,id), UNIQUE(workspace_id,kind,normalized_value)
);
CREATE TABLE contacts.contact_identifiers (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL,
 contact_id uuid NOT NULL, identifier_id uuid NOT NULL,
 original_value text NOT NULL CHECK(length(original_value)<=320),
 created_at timestamptz NOT NULL DEFAULT now(), ended_at timestamptz,
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,identifier_id) REFERENCES contacts.identifiers(workspace_id,id)
);
CREATE UNIQUE INDEX ON contacts.contact_identifiers(workspace_id,contact_id,identifier_id) WHERE ended_at IS NULL;
CREATE INDEX ON contacts.contact_identifiers(workspace_id,identifier_id);
CREATE TABLE suite.idempotency_records (
 workspace_id uuid NOT NULL REFERENCES suite.workspaces(id), actor_user_id uuid NOT NULL REFERENCES suite.users(id),
 operation_key text NOT NULL, idempotency_key text NOT NULL CHECK(length(idempotency_key) BETWEEN 1 AND 128),
 request jsonb NOT NULL CHECK(octet_length(request::text)<=65536), result jsonb NOT NULL CHECK(octet_length(result::text)<=65536),
 created_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL DEFAULT now()+interval '7 days',
 PRIMARY KEY(workspace_id,actor_user_id,operation_key,idempotency_key)
);
ALTER TABLE suite.audit_events ADD COLUMN record_id uuid;
ALTER TABLE suite.audit_events ADD COLUMN request_id uuid;
GRANT SELECT,INSERT,UPDATE,DELETE ON contacts.contacts, contacts.contact_identifiers TO looski_runtime;
GRANT SELECT,INSERT ON contacts.identifiers TO looski_runtime;
GRANT SELECT,INSERT ON suite.idempotency_records TO looski_runtime;
UPDATE suite.schema_version SET version=3;

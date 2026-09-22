-- Shared organizations and temporal person/organization relationships.
INSERT INTO suite.permissions(key,description) VALUES
 ('contacts.organizations.read','Read permitted organizations'),
 ('contacts.organizations.create','Create organizations'),
 ('contacts.organizations.update','Update permitted organizations'),
 ('contacts.organizations.archive','Archive or restore organizations'),
 ('contacts.relationships.manage','Manage a permitted contact organization relationship');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('contacts.organizations.read','contacts.organizations.create',
 'contacts.organizations.update','contacts.organizations.archive','contacts.relationships.manage');
CREATE TABLE contacts.organizations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 200),
 website_url text CHECK(length(website_url)<=2048), notes text NOT NULL DEFAULT '' CHECK(length(notes)<=10000),
 owner_membership_id uuid NOT NULL, archived_at timestamptz,
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 version bigint NOT NULL DEFAULT 1 CHECK(version>0), UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,owner_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON contacts.organizations(workspace_id,created_at,id);
CREATE INDEX ON contacts.organizations(workspace_id,owner_membership_id);
CREATE TABLE contacts.contact_organizations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL,
 contact_id uuid NOT NULL, organization_id uuid NOT NULL,
 relationship_label text NOT NULL DEFAULT '' CHECK(length(relationship_label)<=120),
 job_title text NOT NULL DEFAULT '' CHECK(length(job_title)<=120), is_primary boolean NOT NULL DEFAULT false,
 created_at timestamptz NOT NULL DEFAULT now(), ended_at timestamptz,
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,organization_id) REFERENCES contacts.organizations(workspace_id,id)
);
CREATE UNIQUE INDEX ON contacts.contact_organizations(workspace_id,contact_id,organization_id) WHERE ended_at IS NULL;
CREATE UNIQUE INDEX ON contacts.contact_organizations(workspace_id,contact_id) WHERE is_primary AND ended_at IS NULL;
CREATE INDEX ON contacts.contact_organizations(workspace_id,organization_id);
GRANT SELECT,INSERT,UPDATE ON contacts.organizations, contacts.contact_organizations TO looski_runtime;
UPDATE suite.schema_version SET version=6;

-- Shared tag definitions; typed module-owned assignments retain referential integrity.
INSERT INTO suite.permissions(key,description) VALUES
 ('workspace.tags.read','Read the workspace tag catalog'),
 ('workspace.tags.manage','Create, rename, archive and restore tag definitions'),
 ('contacts.tags.manage','Manage tags on permitted contacts'),
 ('contacts.organizations.tags.manage','Manage tags on permitted organizations');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('workspace.tags.read','workspace.tags.manage','contacts.tags.manage','contacts.organizations.tags.manage');
CREATE TABLE suite.tags (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 80), name_key text NOT NULL,
 color text CHECK(color ~ '^#[0-9a-fA-F]{6}$'), archived_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id), UNIQUE(workspace_id,name_key)
);
CREATE INDEX ON suite.tags(workspace_id,created_at,id);
CREATE TABLE contacts.contact_tags (
 workspace_id uuid NOT NULL, contact_id uuid NOT NULL, tag_id uuid NOT NULL,
 created_by_membership_id uuid NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,contact_id,tag_id),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,tag_id) REFERENCES suite.tags(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON contacts.contact_tags(workspace_id,tag_id);
CREATE TABLE contacts.organization_tags (
 workspace_id uuid NOT NULL, organization_id uuid NOT NULL, tag_id uuid NOT NULL,
 created_by_membership_id uuid NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,organization_id,tag_id),
 FOREIGN KEY(workspace_id,organization_id) REFERENCES contacts.organizations(workspace_id,id),
 FOREIGN KEY(workspace_id,tag_id) REFERENCES suite.tags(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON contacts.organization_tags(workspace_id,tag_id);
GRANT SELECT,INSERT,UPDATE ON suite.tags TO looski_runtime;
GRANT SELECT,INSERT,DELETE ON contacts.contact_tags,contacts.organization_tags TO looski_runtime;
UPDATE suite.schema_version SET version=7;

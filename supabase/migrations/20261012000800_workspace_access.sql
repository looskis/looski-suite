-- Explicit administrative grant authority over the shipped permission catalog.
ALTER TABLE suite.roles ADD COLUMN version bigint NOT NULL DEFAULT 1 CHECK (version>0);
ALTER TABLE suite.roles ADD COLUMN created_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE suite.roles ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE suite.roles ADD CONSTRAINT role_name_length CHECK (length(btrim(name)) BETWEEN 1 AND 120);
-- Reserved, unimplemented grants must not become authority without an explicit later grant.
DELETE FROM suite.role_permissions WHERE permission_key IN ('contacts.delete_permanently','contacts.export');
INSERT INTO suite.permissions VALUES
 ('workspace.access.read','Read workspace staff, roles and supported grants'),
 ('workspace.access.admin','Administer all supported staff role grants and memberships');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,p.key FROM suite.roles CROSS JOIN suite.permissions p
 WHERE suite.roles.key='owner' AND p.key IN ('workspace.access.read','workspace.access.admin');
GRANT UPDATE (version) ON suite.workspaces TO looski_runtime;
GRANT UPDATE (name,archived_at,version,updated_at) ON suite.roles TO looski_runtime;
GRANT UPDATE (state,version,updated_at) ON suite.memberships TO looski_runtime;
GRANT DELETE ON suite.role_permissions,suite.membership_roles TO looski_runtime;
UPDATE suite.schema_version SET version=9;

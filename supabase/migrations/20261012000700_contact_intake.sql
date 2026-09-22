INSERT INTO suite.permissions(key,description) VALUES
 ('contacts.intake.create','Submit input for contact resolution'),
 ('contacts.intake.read','Read permitted contact intake evidence'),
 ('contacts.intake.resolve','Review and resolve permitted contact intake');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('contacts.intake.create','contacts.intake.read','contacts.intake.resolve');
CREATE TABLE contacts.intakes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 owner_membership_id uuid NOT NULL,source_ref text NOT NULL CHECK(length(source_ref) BETWEEN 1 AND 200),
 input jsonb NOT NULL CHECK(octet_length(input::text)<=32768),
 state text NOT NULL CHECK(state IN ('pending','resolved','dismissed')),
 contact_id uuid, resolution_reason text CHECK(length(resolution_reason)<=1000),
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 CHECK((state='resolved')=(contact_id IS NOT NULL)),
 FOREIGN KEY(workspace_id,owner_membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id)
);
CREATE INDEX ON contacts.intakes(workspace_id,state,created_at,id);
CREATE INDEX ON contacts.intakes(workspace_id,owner_membership_id);
CREATE INDEX ON contacts.intakes(workspace_id,contact_id);
GRANT SELECT,INSERT ON contacts.intakes TO looski_runtime;
GRANT UPDATE(state,contact_id,resolution_reason,version,updated_at) ON contacts.intakes TO looski_runtime;
UPDATE suite.schema_version SET version=8;

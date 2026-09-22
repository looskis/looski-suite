-- Workspace-owned invitations. Bearer tokens are never persisted.
CREATE TABLE suite.staff_invitations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 nonce uuid NOT NULL DEFAULT gen_random_uuid(),
 email text NOT NULL CHECK (length(email) BETWEEN 3 AND 254),
 inviter_membership_id uuid NOT NULL,
 inviter_identity_id uuid NOT NULL REFERENCES suite.auth_identities(id),
 expires_at timestamptz NOT NULL,
 state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','accepted','revoked')),
 accepted_membership_id uuid,
 accepted_at timestamptz,
 revoked_at timestamptz,
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(),
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,inviter_membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,accepted_membership_id) REFERENCES suite.memberships(workspace_id,id),
 CHECK ((state='accepted')=(accepted_membership_id IS NOT NULL AND accepted_at IS NOT NULL)),
 CHECK ((state='revoked')=(revoked_at IS NOT NULL)),
 CHECK (expires_at>created_at)
);
CREATE INDEX ON suite.staff_invitations(workspace_id,created_at,id);
CREATE TABLE suite.staff_invitation_roles (
 workspace_id uuid NOT NULL,
 invitation_id uuid NOT NULL,
 role_id uuid NOT NULL,
 role_version bigint NOT NULL CHECK(role_version>0),
 role_name text NOT NULL,
 PRIMARY KEY(workspace_id,invitation_id,role_id),
 FOREIGN KEY(workspace_id,invitation_id) REFERENCES suite.staff_invitations(workspace_id,id),
 FOREIGN KEY(workspace_id,role_id) REFERENCES suite.roles(workspace_id,id)
);
GRANT SELECT,INSERT,UPDATE ON suite.staff_invitations TO looski_runtime;
GRANT SELECT,INSERT ON suite.staff_invitation_roles TO looski_runtime;
UPDATE suite.schema_version SET version=24;

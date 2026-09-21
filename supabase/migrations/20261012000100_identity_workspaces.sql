-- First owner and authenticated workspace discovery. Supabase owns auth.*.
CREATE TABLE suite.users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    display_name text NOT NULL CHECK (length(btrim(display_name)) BETWEEN 1 AND 120),
    state text NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'suspended')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    version bigint NOT NULL DEFAULT 1 CHECK (version > 0)
);
CREATE TABLE suite.auth_identities (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES suite.users(id),
    issuer text NOT NULL,
    subject text NOT NULL,
    linked_at timestamptz NOT NULL DEFAULT now(),
    disabled_at timestamptz,
    UNIQUE (issuer, subject)
);
CREATE INDEX ON suite.auth_identities(user_id);
CREATE TABLE suite.workspaces (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 120),
    state text NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'suspended')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    version bigint NOT NULL DEFAULT 1 CHECK (version > 0)
);
CREATE TABLE suite.memberships (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
    user_id uuid NOT NULL REFERENCES suite.users(id),
    state text NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'suspended', 'removed')),
    joined_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    version bigint NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (workspace_id, user_id),
    UNIQUE (workspace_id, id)
);
CREATE INDEX ON suite.memberships(user_id, workspace_id);
CREATE TABLE suite.permissions (key text PRIMARY KEY, description text NOT NULL);
INSERT INTO suite.permissions VALUES ('workspace.read', 'View workspace identity');
CREATE TABLE suite.roles (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
    key text NOT NULL,
    name text NOT NULL,
    archived_at timestamptz,
    UNIQUE (workspace_id, key),
    UNIQUE (workspace_id, id)
);
CREATE TABLE suite.role_permissions (
    workspace_id uuid NOT NULL,
    role_id uuid NOT NULL,
    permission_key text NOT NULL REFERENCES suite.permissions(key),
    PRIMARY KEY (workspace_id, role_id, permission_key),
    FOREIGN KEY (workspace_id, role_id) REFERENCES suite.roles(workspace_id, id)
);
CREATE INDEX ON suite.role_permissions(permission_key);
CREATE TABLE suite.membership_roles (
    workspace_id uuid NOT NULL,
    membership_id uuid NOT NULL,
    role_id uuid NOT NULL,
    PRIMARY KEY (workspace_id, membership_id, role_id),
    FOREIGN KEY (workspace_id, membership_id) REFERENCES suite.memberships(workspace_id, id),
    FOREIGN KEY (workspace_id, role_id) REFERENCES suite.roles(workspace_id, id)
);
CREATE INDEX ON suite.membership_roles(workspace_id, role_id);
CREATE TABLE suite.audit_events (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
    occurred_at timestamptz NOT NULL DEFAULT now(),
    operation_id uuid NOT NULL,
    actor_user_id uuid NOT NULL REFERENCES suite.users(id),
    action text NOT NULL,
    details jsonb NOT NULL DEFAULT '{}' CHECK (octet_length(details::text) <= 8192)
);
CREATE INDEX ON suite.audit_events(workspace_id, occurred_at, id);
CREATE INDEX ON suite.audit_events(actor_user_id);
-- Durable latch: bootstrap must never reopen because memberships were removed.
CREATE TABLE suite.installation (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    bootstrapped_at timestamptz
);
INSERT INTO suite.installation DEFAULT VALUES;

GRANT SELECT, INSERT ON suite.users, suite.auth_identities, suite.workspaces,
    suite.memberships, suite.roles, suite.role_permissions, suite.membership_roles
    TO looski_runtime;
GRANT SELECT ON suite.permissions, suite.installation TO looski_runtime;
GRANT UPDATE (bootstrapped_at) ON suite.installation TO looski_runtime;
GRANT INSERT ON suite.audit_events TO looski_runtime;
UPDATE suite.schema_version SET version = 2;

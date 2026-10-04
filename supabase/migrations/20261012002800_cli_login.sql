-- First-party CLI/MCP sign-in. A browser session approves a short-lived,
-- single-use code bound to a PKCE S256 challenge. Only the code's SHA-256 is
-- stored; consumed and expired rows are retained as sign-in history.
CREATE TABLE suite.cli_authorizations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 user_id uuid NOT NULL REFERENCES suite.users(id),
 identity_id uuid NOT NULL REFERENCES suite.auth_identities(id),
 code_hash bytea NOT NULL UNIQUE CHECK(octet_length(code_hash)=32),
 code_challenge text NOT NULL CHECK(code_challenge ~ '^[A-Za-z0-9_-]{43}$'),
 device_label text CHECK(length(device_label) BETWEEN 1 AND 64),
 created_at timestamptz NOT NULL DEFAULT now(),
 expires_at timestamptz NOT NULL,
 consumed_at timestamptz,
 outcome text CHECK(outcome IN ('issued','rejected')),
 session_id uuid,
 CHECK(expires_at>created_at AND expires_at<=created_at+interval '10 minutes'),
 CHECK((consumed_at IS NULL)=(outcome IS NULL)),
 CHECK((outcome='issued')=(session_id IS NOT NULL))
);
CREATE INDEX ON suite.cli_authorizations(user_id,expires_at) WHERE consumed_at IS NULL;
CREATE INDEX ON suite.cli_authorizations(session_id) WHERE session_id IS NOT NULL;
-- User-level security history; workspace business audit stays in audit_events.
CREATE TABLE suite.security_events (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 occurred_at timestamptz NOT NULL DEFAULT now(),
 user_id uuid NOT NULL REFERENCES suite.users(id),
 action text NOT NULL CHECK(action IN ('cli_login.authorized','cli_login.exchanged','cli_login.rejected','cli_session.logout')),
 request_id uuid NOT NULL,
 source_interface text NOT NULL CHECK(source_interface IN ('api','web','cli','mcp')),
 details jsonb NOT NULL DEFAULT '{}' CHECK(octet_length(details::text)<=2048)
);
CREATE INDEX ON suite.security_events(user_id,occurred_at,id);
GRANT SELECT,INSERT ON suite.cli_authorizations TO looski_runtime;
GRANT UPDATE(consumed_at,outcome,session_id) ON suite.cli_authorizations TO looski_runtime;
GRANT INSERT ON suite.security_events TO looski_runtime;
UPDATE suite.schema_version SET version=29;

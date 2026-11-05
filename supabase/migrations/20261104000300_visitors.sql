-- First-party visitor attribution is optional; fingerprints never grant identity/access.
CREATE SCHEMA visitors;
REVOKE ALL ON SCHEMA visitors FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA visitors TO looski_runtime;
CREATE TABLE visitors.sites (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL, public_key text NOT NULL UNIQUE, origins jsonb NOT NULL,
 fingerprint_enabled boolean NOT NULL DEFAULT false, similarity_enabled boolean NOT NULL DEFAULT false,
 threshold double precision NOT NULL DEFAULT 0.95 CHECK(threshold BETWEEN 0.8 AND 1),
 retention_days integer NOT NULL DEFAULT 90 CHECK(retention_days BETWEEN 7 AND 180),
 enabled boolean NOT NULL DEFAULT true, version bigint NOT NULL DEFAULT 1,
 UNIQUE(workspace_id,id)
);
CREATE TABLE visitors.visitors (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL, site_id uuid NOT NULL,
 token_hash text NOT NULL, components jsonb NOT NULL DEFAULT '{}', fingerprint text,
 recipe text NOT NULL DEFAULT 'browser-v1', contact_id uuid, evidence text,
 first_seen timestamptz NOT NULL DEFAULT now(), last_seen timestamptz NOT NULL DEFAULT now(),
 expires_at timestamptz NOT NULL, version bigint NOT NULL DEFAULT 1,
 UNIQUE(workspace_id,id), UNIQUE(site_id,token_hash),
 FOREIGN KEY(workspace_id,site_id) REFERENCES visitors.sites(workspace_id,id) ON DELETE CASCADE,
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id) ON DELETE CASCADE
);
CREATE INDEX ON visitors.visitors(site_id,last_seen);
CREATE INDEX ON visitors.visitors(expires_at);
CREATE INDEX ON visitors.visitors(workspace_id,contact_id);
CREATE TABLE visitors.events (
 id uuid NOT NULL, workspace_id uuid NOT NULL, site_id uuid NOT NULL, visitor_id uuid NOT NULL,
 page text NOT NULL, referrer_host text, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(site_id,id),
 FOREIGN KEY(workspace_id,visitor_id) REFERENCES visitors.visitors(workspace_id,id) ON DELETE CASCADE
);
CREATE INDEX ON visitors.events(visitor_id,created_at);
CREATE TABLE visitors.matches (
 workspace_id uuid NOT NULL, visitor_id uuid NOT NULL, candidate_id uuid NOT NULL,
 score double precision NOT NULL CHECK(score BETWEEN 0 AND 1), reason text NOT NULL,
 state text NOT NULL DEFAULT 'suggested' CHECK(state IN ('suggested','accepted','rejected')),
 version bigint NOT NULL DEFAULT 1, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,visitor_id,candidate_id), CHECK(visitor_id<>candidate_id),
 FOREIGN KEY(workspace_id,visitor_id) REFERENCES visitors.visitors(workspace_id,id) ON DELETE CASCADE,
 FOREIGN KEY(workspace_id,candidate_id) REFERENCES visitors.visitors(workspace_id,id) ON DELETE CASCADE
);
REVOKE ALL ON visitors.sites,visitors.visitors,visitors.events,visitors.matches FROM PUBLIC,anon,authenticated;
GRANT SELECT,INSERT,UPDATE,DELETE ON visitors.sites,visitors.visitors,visitors.events,visitors.matches TO looski_runtime;
-- Short-lived revocations prevent in-flight requests from resurrecting withdrawn data.
CREATE TABLE visitors.withdrawals (
 site_id uuid NOT NULL REFERENCES visitors.sites(id) ON DELETE CASCADE,
 token_hash text NOT NULL, expires_at timestamptz NOT NULL DEFAULT now()+interval '180 days',
 PRIMARY KEY(site_id,token_hash)
);
REVOKE ALL ON visitors.withdrawals FROM PUBLIC,anon,authenticated;
GRANT SELECT,INSERT,UPDATE,DELETE ON visitors.withdrawals TO looski_runtime;
CREATE INDEX ON visitors.withdrawals(expires_at);
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','communications','forms','cms','projects','knowledge','helpdesk','time','scheduling','visitors'));
INSERT INTO suite.permissions(key,description) VALUES
 ('visitors.read','Read workspace visitor attribution and evidence'),
 ('visitors.manage','Resolve declared visitor attribution, review suggestions, export and delete visitors'),
 ('visitors.configure','Configure first-party sites and optional browser matching');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p WHERE r.key='owner' AND p.key LIKE 'visitors.%';
UPDATE suite.schema_version SET version=40;

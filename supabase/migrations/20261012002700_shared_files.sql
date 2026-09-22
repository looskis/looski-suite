-- Shared bounded Files. Content and metadata commit with ordinary business audit.
CREATE SCHEMA files;
CREATE TABLE files.files (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 owner_membership_id uuid NOT NULL,
 filename text NOT NULL CHECK(length(filename) BETWEEN 1 AND 255 AND filename=btrim(filename) AND filename NOT IN ('.','..') AND position('/' in filename)=0 AND position(chr(92) in filename)=0 AND filename !~ '[[:cntrl:]]'),
 media_type text NOT NULL CHECK(length(media_type)<=127 AND media_type ~ '^[A-Za-z0-9!#$&^_.+-]+/[A-Za-z0-9!#$&^_.+-]+$'),
 notes text NOT NULL DEFAULT '' CHECK(length(notes)<=10000),
 byte_length bigint NOT NULL CHECK(byte_length BETWEEN 0 AND 8388608),
 sha256 text NOT NULL CHECK(sha256 ~ '^[0-9a-f]{64}$'),
 archived_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,id,sha256,byte_length),
 FOREIGN KEY(workspace_id,owner_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON files.files(workspace_id,created_at,id);
CREATE TABLE files.contents (
 workspace_id uuid NOT NULL,file_id uuid NOT NULL,
 sha256 text NOT NULL,byte_length bigint NOT NULL,
 content bytea NOT NULL CHECK(octet_length(content)<=8388608),
 PRIMARY KEY(workspace_id,file_id),
 FOREIGN KEY(workspace_id,file_id,sha256,byte_length) REFERENCES files.files(workspace_id,id,sha256,byte_length),
 CHECK(octet_length(content)=byte_length AND encode(pg_catalog.sha256(content),'hex')=sha256)
);
GRANT USAGE ON SCHEMA files TO looski_runtime;
GRANT SELECT,INSERT ON files.files,files.contents TO looski_runtime;
GRANT UPDATE(filename,media_type,notes,owner_membership_id,archived_at,version,updated_at) ON files.files TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('files.read','Read permitted file metadata and immutable content'),
 ('files.create','Upload bounded files'),('files.update','Edit file metadata and responsibility'),
 ('files.archive','Archive and restore permitted files');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('files.read','files.create','files.update','files.archive');
UPDATE suite.schema_version SET version=28;

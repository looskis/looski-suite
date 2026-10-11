-- Optional CMS: content types, editable entry drafts, immutable revisions, and
-- media that gives chosen Files a public address only through published content.
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','forms','cms'));
CREATE SCHEMA cms;
REVOKE ALL ON SCHEMA cms FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA cms TO looski_runtime;
CREATE TABLE cms.content_types (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 description text NOT NULL DEFAULT '' CHECK(length(description)<=2000),
 fields jsonb NOT NULL CHECK(jsonb_typeof(fields)='array' AND octet_length(fields::text)<=65536),
 -- Installation-unique bearer of public read rights for this type's published entries only.
 public_key text NOT NULL UNIQUE CHECK(public_key ~ '^[A-Za-z0-9_-]{22}$'),
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id)
);
CREATE INDEX ON cms.content_types(workspace_id,created_at,id);
CREATE TABLE cms.media (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,file_id uuid NOT NULL,
 title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 200),
 alt_text text NOT NULL DEFAULT '' CHECK(length(alt_text)<=1000),
 -- Checked against the bytes at registration; later Files metadata edits cannot change how it is served.
 media_type text NOT NULL CHECK(media_type IN ('image/png','image/jpeg','image/gif','image/webp','application/pdf')),
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,file_id),
 FOREIGN KEY(workspace_id,file_id) REFERENCES files.files(workspace_id,id)
);
CREATE INDEX ON cms.media(workspace_id,created_at,id);
CREATE TABLE cms.entries (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,type_id uuid NOT NULL,
 title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 200),
 slug text NOT NULL CHECK(slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$' AND length(slug)<=120),
 draft_values jsonb NOT NULL CHECK(jsonb_typeof(draft_values)='object' AND octet_length(draft_values::text)<=524288),
 published_revision_id uuid,public_slug text,published_at timestamptz,
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 -- One live entry per public address; drafts may share a slug until publication.
 UNIQUE(workspace_id,type_id,public_slug),
 FOREIGN KEY(workspace_id,type_id) REFERENCES cms.content_types(workspace_id,id),
 CHECK((published_revision_id IS NULL)=(public_slug IS NULL) AND (published_revision_id IS NULL)=(published_at IS NULL)),
 CHECK(archived_at IS NULL OR published_revision_id IS NULL)
);
CREATE INDEX ON cms.entries(workspace_id,created_at,id);
CREATE INDEX ON cms.entries(workspace_id,type_id,public_slug);
CREATE TABLE cms.revisions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,entry_id uuid NOT NULL,
 number integer NOT NULL CHECK(number>0),title text NOT NULL,slug text NOT NULL,
 content jsonb NOT NULL CHECK(jsonb_typeof(content)='object' AND octet_length(content::text)<=524288),
 -- The type's fields when this revision was made, so later type edits never change it.
 fields jsonb NOT NULL CHECK(jsonb_typeof(fields)='array'),
 restored_from_number integer,
 created_by_membership_id uuid NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 last_published_at timestamptz,
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,entry_id,id),UNIQUE(workspace_id,entry_id,number),
 FOREIGN KEY(workspace_id,entry_id) REFERENCES cms.entries(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
ALTER TABLE cms.entries ADD FOREIGN KEY(workspace_id,id,published_revision_id) REFERENCES cms.revisions(workspace_id,entry_id,id);
-- The only path from a public key to bytes: media named by a published revision.
CREATE TABLE cms.revision_media (
 workspace_id uuid NOT NULL,revision_id uuid NOT NULL,media_id uuid NOT NULL,
 PRIMARY KEY(workspace_id,revision_id,media_id),
 FOREIGN KEY(workspace_id,revision_id) REFERENCES cms.revisions(workspace_id,id),
 FOREIGN KEY(workspace_id,media_id) REFERENCES cms.media(workspace_id,id)
);
CREATE INDEX ON cms.revision_media(workspace_id,media_id);
GRANT SELECT,INSERT ON cms.content_types,cms.media,cms.entries,cms.revisions,cms.revision_media TO looski_runtime;
GRANT UPDATE(name,description,fields,archived_at,version,updated_at) ON cms.content_types TO looski_runtime;
GRANT UPDATE(title,alt_text,archived_at,version,updated_at) ON cms.media TO looski_runtime;
GRANT UPDATE(title,slug,draft_values,published_revision_id,public_slug,published_at,archived_at,version,updated_at) ON cms.entries TO looski_runtime;
GRANT UPDATE(last_published_at) ON cms.revisions TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('cms.read','Read content types, media and published entries'),
 ('cms.drafts.read','Read entry drafts, previews and revision history'),
 ('cms.types.manage','Create, edit and archive content types'),
 ('cms.entries.edit','Create, edit, restore and archive entry drafts'),
 ('cms.entries.publish','Publish and unpublish entries'),
 ('cms.media.manage','Register Files as CMS media and edit or archive media');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('cms.read','cms.drafts.read','cms.types.manage','cms.entries.edit','cms.entries.publish','cms.media.manage');
UPDATE suite.schema_version SET version=31;

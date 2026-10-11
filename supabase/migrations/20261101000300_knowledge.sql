-- Optional knowledge base: collections, documents, immutable revisions and PostgreSQL
-- full-text search. Nothing here depends on CMS or any other optional application.
ALTER TABLE suite.workspace_modules DROP CONSTRAINT workspace_modules_module_key_check,
 ADD CONSTRAINT workspace_modules_module_key_check CHECK(module_key IN ('crm','forms','cms','projects','knowledge'));
CREATE SCHEMA knowledge;
REVOKE ALL ON SCHEMA knowledge FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA knowledge TO looski_runtime;
-- Names are deliberately not unique: a refused duplicate would reveal a hidden collection.
CREATE TABLE knowledge.collections (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 description text NOT NULL DEFAULT '' CHECK(length(description)<=2000),
 -- A restricted collection is reachable only through its member list or the explicit override.
 restricted boolean NOT NULL DEFAULT false,
 created_by_membership_id uuid NOT NULL,archived_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON knowledge.collections(workspace_id,created_at,id);
CREATE TABLE knowledge.collection_members (
 workspace_id uuid NOT NULL,collection_id uuid NOT NULL,membership_id uuid NOT NULL,
 added_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,collection_id,membership_id),
 FOREIGN KEY(workspace_id,collection_id) REFERENCES knowledge.collections(workspace_id,id),
 FOREIGN KEY(workspace_id,membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON knowledge.collection_members(workspace_id,membership_id,collection_id);
-- Titles are not unique and documents have no slugs, for the same reason as collection names.
CREATE TABLE knowledge.documents (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,collection_id uuid NOT NULL,
 kind text NOT NULL DEFAULT 'article' CHECK(kind IN ('article','sop')),
 title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 200),
 -- Plain source; only the typed blocks parsed from it reach a renderer.
 body text NOT NULL DEFAULT '' CHECK(length(body)<=100000),
 -- The words a reader sees, from those blocks: no markup and no link targets.
 search_text text NOT NULL DEFAULT '',
 -- Only the current text is indexed, so history is never searchable. ts_rank reads
 -- this row alone and keeps no corpus statistics across documents.
 search_vector tsvector GENERATED ALWAYS AS (setweight(to_tsvector('english'::regconfig,title),'A')||setweight(to_tsvector('english'::regconfig,search_text),'B')) STORED,
 head_number integer NOT NULL DEFAULT 1 CHECK(head_number>0),archived_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,collection_id) REFERENCES knowledge.collections(workspace_id,id)
);
CREATE INDEX documents_search ON knowledge.documents USING gin(search_vector);
CREATE INDEX ON knowledge.documents(workspace_id,created_at,id);
CREATE INDEX ON knowledge.documents(workspace_id,collection_id,created_at,id);
-- Immutable: the runtime role can add a revision and nothing else. collection_id records
-- where the text was written; reading it needs that collection as well as the document's
-- current one, so moving a document does not release the history it had elsewhere.
CREATE TABLE knowledge.revisions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,document_id uuid NOT NULL,
 number integer NOT NULL CHECK(number>0),collection_id uuid NOT NULL,
 change text NOT NULL CHECK(change IN ('created','edited','moved','restored')),
 kind text NOT NULL CHECK(kind IN ('article','sop')),
 title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 200),
 body text NOT NULL CHECK(length(body)<=100000),
 restored_from_number integer,created_by_membership_id uuid NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,document_id,number),
 FOREIGN KEY(workspace_id,document_id) REFERENCES knowledge.documents(workspace_id,id),
 FOREIGN KEY(workspace_id,collection_id) REFERENCES knowledge.collections(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,document_id,restored_from_number) REFERENCES knowledge.revisions(workspace_id,document_id,number),
 CHECK((change='restored')=(restored_from_number IS NOT NULL))
);
CREATE INDEX ON knowledge.revisions(workspace_id,created_at,id);
-- The one definition of who reaches a collection. Written as a single expression so the
-- planner inlines it. p_all is the override, p_wide a workspace-wide grant of the action.
CREATE FUNCTION knowledge.reaches(p_workspace uuid,p_collection uuid,p_all boolean,p_wide boolean,p_member uuid)
RETURNS boolean LANGUAGE sql STABLE AS $$
 SELECT p_all
  OR (p_wide AND NOT (SELECT c.restricted FROM knowledge.collections c WHERE c.workspace_id=p_workspace AND c.id=p_collection))
  OR EXISTS(SELECT 1 FROM knowledge.collection_members m WHERE m.workspace_id=p_workspace AND m.collection_id=p_collection AND m.membership_id=p_member)
$$;
REVOKE ALL ON FUNCTION knowledge.reaches(uuid,uuid,boolean,boolean,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION knowledge.reaches(uuid,uuid,boolean,boolean,uuid) TO looski_runtime;
GRANT SELECT,INSERT ON knowledge.collections,knowledge.documents,knowledge.revisions TO looski_runtime;
GRANT SELECT,INSERT,DELETE ON knowledge.collection_members TO looski_runtime;
GRANT UPDATE(name,description,restricted,archived_at,version,updated_at) ON knowledge.collections TO looski_runtime;
GRANT UPDATE(collection_id,kind,title,body,search_text,head_number,archived_at,version,updated_at) ON knowledge.documents TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES
 ('knowledge.read','Find and read documents; workspace scope reaches open collections and those the member is listed on, assigned scope only those they are listed on'),
 ('knowledge.documents.edit','Create, edit, move, archive and restore documents in collections the member reaches'),
 ('knowledge.collections.manage','Create, edit and archive collections and manage their members; creating needs workspace scope'),
 ('knowledge.restricted.access','Reach every restricted collection without being listed on it, for any knowledge permission also held');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('knowledge.read','knowledge.documents.edit','knowledge.collections.manage','knowledge.restricted.access');
UPDATE suite.schema_version SET version=33;

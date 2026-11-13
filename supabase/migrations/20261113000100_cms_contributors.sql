-- Invited contributors use a dedicated permission plus explicit collection grants.
-- Existing CMS permissions remain workspace-wide; no staff grants are implied.
INSERT INTO suite.permissions(key,description) VALUES ('cms.contribute','Write own articles in approved CMS collections');
CREATE TABLE cms.contributors (
 workspace_id uuid NOT NULL, membership_id uuid NOT NULL, type_id uuid NOT NULL,
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
 bio text NOT NULL DEFAULT '' CHECK(length(bio)<=2000), photo_media_id uuid,
 active boolean NOT NULL DEFAULT true, version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 PRIMARY KEY(workspace_id,membership_id,type_id),
 FOREIGN KEY(workspace_id,membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,type_id) REFERENCES cms.content_types(workspace_id,id),
 FOREIGN KEY(workspace_id,photo_media_id) REFERENCES cms.media(workspace_id,id)
);
ALTER TABLE cms.content_types ADD COLUMN contribution_coauthor jsonb;
ALTER TABLE cms.entries ADD COLUMN contributor_id uuid,
 ADD COLUMN authors jsonb NOT NULL DEFAULT '[]',
 ADD COLUMN review_state text NOT NULL DEFAULT 'draft' CHECK(review_state IN ('draft','submitted','changes_requested','published')),
 ADD COLUMN submitted_revision_id uuid,
 ADD COLUMN review_note text NOT NULL DEFAULT '' CHECK(length(review_note)<=3000),
 ADD FOREIGN KEY(workspace_id,contributor_id) REFERENCES suite.memberships(workspace_id,id),
 ADD FOREIGN KEY(workspace_id,id,submitted_revision_id) REFERENCES cms.revisions(workspace_id,entry_id,id);
CREATE INDEX ON cms.entries(workspace_id,contributor_id,created_at,id);
CREATE INDEX ON cms.entries(workspace_id,review_state,created_at,id) WHERE contributor_id IS NOT NULL;
ALTER TABLE cms.revisions ADD COLUMN authors jsonb NOT NULL DEFAULT '[]';
ALTER TABLE cms.media ADD COLUMN contributor_id uuid,
 ADD FOREIGN KEY(workspace_id,contributor_id) REFERENCES suite.memberships(workspace_id,id);
CREATE TABLE cms.contribution_events (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL, entry_id uuid NOT NULL,
 kind text NOT NULL CHECK(kind IN ('submitted','changes_requested','published')),
 note text NOT NULL DEFAULT '', created_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,entry_id) REFERENCES cms.entries(workspace_id,id)
);
CREATE INDEX ON cms.contribution_events(workspace_id,created_at DESC,id);
GRANT SELECT,INSERT,UPDATE ON cms.contributors TO looski_runtime;
GRANT SELECT,INSERT ON cms.contribution_events TO looski_runtime;
GRANT UPDATE(contribution_coauthor) ON cms.content_types TO looski_runtime;
GRANT UPDATE(contributor_id,authors,review_state,submitted_revision_id,review_note) ON cms.entries TO looski_runtime;
GRANT UPDATE(contributor_id) ON cms.media TO looski_runtime;
UPDATE suite.schema_version SET version=41;

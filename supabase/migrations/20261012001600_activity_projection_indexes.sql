-- Applied activity tables remain intact; support configured order and alias families.
CREATE INDEX ON crm.activity_types(workspace_id,(archived_at IS NOT NULL),position,created_at,id);
CREATE INDEX ON contacts.contacts(workspace_id,merged_into_id,id) WHERE merged_into_id IS NOT NULL;
UPDATE suite.schema_version SET version=17;

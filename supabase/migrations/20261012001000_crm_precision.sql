-- Reconcile the first CRM increment with the canonical exact estimate contract.
-- PostgreSQL refuses conversion if any old amount exceeds the supported range;
-- never round or truncate a historical value to make an upgrade pass.
ALTER TABLE crm.opportunities ALTER COLUMN expected_value TYPE numeric(20,6);
ALTER TABLE crm.deals ALTER COLUMN value TYPE numeric(20,6);
CREATE INDEX ON crm.opportunities(workspace_id,created_at,id);
CREATE INDEX ON crm.opportunities(workspace_id,owner_membership_id,created_at,id);
CREATE INDEX ON crm.deals(workspace_id,created_at,id);
CREATE INDEX ON crm.deals(workspace_id,owner_membership_id,created_at,id);
CREATE INDEX ON suite.roles(workspace_id,created_at,id);
CREATE INDEX ON suite.memberships(workspace_id,joined_at,id);
UPDATE suite.schema_version SET version=11;

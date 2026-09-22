-- Retain minimal attribution/retry proof, never a copy of the deleted contact.
CREATE TABLE contacts.deletion_receipts (
 workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),id uuid NOT NULL,
 owner_membership_id uuid NOT NULL,deleted_version bigint NOT NULL CHECK(deleted_version>0),
 deleted_at timestamptz NOT NULL DEFAULT clock_timestamp(),PRIMARY KEY(workspace_id,id),
 FOREIGN KEY(workspace_id,owner_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
GRANT SELECT,INSERT ON contacts.deletion_receipts TO looski_runtime;
GRANT DELETE ON contacts.identifiers TO looski_runtime;
ALTER TABLE suite.idempotency_records ADD COLUMN invalidated_at timestamptz;
GRANT UPDATE(request,result,invalidated_at) ON suite.idempotency_records TO looski_runtime;
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,'contacts.delete_permanently' FROM suite.roles WHERE key='owner';
UPDATE suite.schema_version SET version=26;

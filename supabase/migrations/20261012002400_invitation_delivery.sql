-- Invitation delivery shares durable Jobs, but payloads remain Access-owned.
ALTER TABLE suite.jobs DROP CONSTRAINT jobs_kind_check,
 ADD CONSTRAINT jobs_kind_check CHECK(kind IN ('crm.coordination','workspace.invitation_email'));
ALTER TABLE suite.jobs DROP CONSTRAINT jobs_error_code_check,
 ADD CONSTRAINT jobs_error_code_check CHECK(error_code IS NULL OR error_code IN ('authority','unavailable','invalid_input','database','lease_expired','module_disabled','provider'));
CREATE TABLE suite.invitation_deliveries (
 workspace_id uuid NOT NULL,job_id uuid PRIMARY KEY,invitation_id uuid NOT NULL,
 FOREIGN KEY(workspace_id,job_id) REFERENCES suite.jobs(workspace_id,id),
 FOREIGN KEY(workspace_id,invitation_id) REFERENCES suite.staff_invitations(workspace_id,id)
);
CREATE INDEX ON suite.invitation_deliveries(workspace_id,invitation_id);
GRANT SELECT,INSERT ON suite.invitation_deliveries TO looski_runtime;
UPDATE suite.schema_version SET version=25;

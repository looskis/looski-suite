-- Durable provider-neutral calendar delivery; no OAuth credentials in the database.
CREATE TABLE scheduling.calendar_jobs (
    workspace_id uuid NOT NULL,
    job_id uuid NOT NULL,
    appointment_id uuid NOT NULL,
    PRIMARY KEY (workspace_id, job_id),
    FOREIGN KEY (workspace_id, job_id) REFERENCES suite.jobs(workspace_id,id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, appointment_id) REFERENCES scheduling.appointments(workspace_id,id) ON DELETE CASCADE
);

REVOKE ALL ON scheduling.calendar_jobs FROM PUBLIC,anon,authenticated;
GRANT SELECT,INSERT ON scheduling.calendar_jobs TO looski_runtime;
ALTER TABLE suite.jobs DROP CONSTRAINT jobs_kind_check,
 ADD CONSTRAINT jobs_kind_check CHECK(kind IN ('crm.coordination','workspace.invitation_email','forms.crm_inquiry','mail.notification','projects.deal_conversion','scheduling.crm_inquiry','scheduling.reminder','communications.send','scheduling.calendar'));
UPDATE suite.schema_version SET version=39;

-- Headless collections keep the original website as their sole HTML surface.
ALTER TABLE cms.content_types ADD COLUMN headless boolean NOT NULL DEFAULT false;
GRANT UPDATE(headless) ON cms.content_types TO looski_runtime;

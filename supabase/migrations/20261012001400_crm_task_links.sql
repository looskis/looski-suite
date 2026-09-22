-- CRM-owned task links retain history without copying task state.
CREATE TABLE crm.opportunity_tasks (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL,
 opportunity_id uuid NOT NULL, task_id uuid NOT NULL, created_by_membership_id uuid NOT NULL,
 ended_at timestamptz, created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id), UNIQUE(workspace_id,opportunity_id,task_id),
 FOREIGN KEY(workspace_id,opportunity_id) REFERENCES crm.opportunities(workspace_id,id),
 FOREIGN KEY(workspace_id,task_id) REFERENCES tasks.tasks(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON crm.opportunity_tasks(workspace_id,opportunity_id,created_at,id);
CREATE INDEX ON crm.opportunity_tasks(workspace_id,task_id);
CREATE INDEX ON crm.opportunity_tasks(workspace_id,created_by_membership_id);
CREATE TABLE crm.opportunity_task_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL,link_id uuid NOT NULL,
 ended boolean NOT NULL,audit_event_id uuid NOT NULL,occurred_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,link_id) REFERENCES crm.opportunity_tasks(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.opportunity_task_changes(workspace_id,link_id,occurred_at,id);
CREATE INDEX ON crm.opportunity_task_changes(workspace_id,audit_event_id);
GRANT SELECT,INSERT,UPDATE ON crm.opportunity_tasks TO looski_runtime;
GRANT SELECT,INSERT ON crm.opportunity_task_changes TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES('crm.opportunities.tasks.manage','Manage task links and follow-ups on permitted opportunities');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,'crm.opportunities.tasks.manage' FROM suite.roles WHERE key='owner';
CREATE TABLE crm.deal_tasks (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL,
 deal_id uuid NOT NULL, task_id uuid NOT NULL, created_by_membership_id uuid NOT NULL,
 ended_at timestamptz, created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id), UNIQUE(workspace_id,deal_id,task_id),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,task_id) REFERENCES tasks.tasks(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON crm.deal_tasks(workspace_id,deal_id,created_at,id);
CREATE INDEX ON crm.deal_tasks(workspace_id,task_id);
CREATE INDEX ON crm.deal_tasks(workspace_id,created_by_membership_id);
CREATE TABLE crm.deal_task_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL,link_id uuid NOT NULL,
 ended boolean NOT NULL,audit_event_id uuid NOT NULL,occurred_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,link_id) REFERENCES crm.deal_tasks(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.deal_task_changes(workspace_id,link_id,occurred_at,id);
CREATE INDEX ON crm.deal_task_changes(workspace_id,audit_event_id);
GRANT SELECT,INSERT,UPDATE ON crm.deal_tasks TO looski_runtime;
GRANT SELECT,INSERT ON crm.deal_task_changes TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES('crm.deals.tasks.manage','Manage task links and follow-ups on permitted deals');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,'crm.deals.tasks.manage' FROM suite.roles WHERE key='owner';
UPDATE suite.schema_version SET version=15;

-- CRM-owned typed assignments over the shared tag catalog.
CREATE TABLE crm.opportunity_tags (
 workspace_id uuid NOT NULL, opportunity_id uuid NOT NULL, tag_id uuid NOT NULL,
 created_by_membership_id uuid NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,opportunity_id,tag_id),
 FOREIGN KEY(workspace_id,opportunity_id) REFERENCES crm.opportunities(workspace_id,id),
 FOREIGN KEY(workspace_id,tag_id) REFERENCES suite.tags(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON crm.opportunity_tags(workspace_id,tag_id,opportunity_id);
GRANT SELECT,INSERT,DELETE ON crm.opportunity_tags TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES('crm.opportunities.tags.manage','Manage tags on permitted opportunities');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,'crm.opportunities.tags.manage' FROM suite.roles WHERE key='owner';
CREATE TABLE crm.deal_tags (
 workspace_id uuid NOT NULL, deal_id uuid NOT NULL, tag_id uuid NOT NULL,
 created_by_membership_id uuid NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,deal_id,tag_id),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,tag_id) REFERENCES suite.tags(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE INDEX ON crm.deal_tags(workspace_id,tag_id,deal_id);
GRANT SELECT,INSERT,DELETE ON crm.deal_tags TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES('crm.deals.tags.manage','Manage tags on permitted deals');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,'crm.deals.tags.manage' FROM suite.roles WHERE key='owner';
UPDATE suite.schema_version SET version=12;

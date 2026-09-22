-- CRM owns participant associations and immutable metadata snapshots.
CREATE TABLE crm.opportunity_contacts (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,
 opportunity_id uuid NOT NULL,contact_id uuid NOT NULL,
 role_label text NOT NULL DEFAULT '' CHECK(length(role_label)<=80),is_primary boolean NOT NULL DEFAULT false,
 ended_at timestamptz,created_by_membership_id uuid NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK(NOT is_primary OR ended_at IS NULL),UNIQUE(workspace_id,id),UNIQUE(workspace_id,opportunity_id,contact_id),
 FOREIGN KEY(workspace_id,opportunity_id) REFERENCES crm.opportunities(workspace_id,id),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE UNIQUE INDEX ON crm.opportunity_contacts(workspace_id,opportunity_id) WHERE is_primary AND ended_at IS NULL;
CREATE INDEX ON crm.opportunity_contacts(workspace_id,opportunity_id,created_at,id);
CREATE INDEX ON crm.opportunity_contacts(workspace_id,contact_id);
CREATE TABLE crm.opportunity_contact_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,association_id uuid NOT NULL,
 role_label text NOT NULL,is_primary boolean NOT NULL,ended boolean NOT NULL,
 occurred_at timestamptz NOT NULL DEFAULT now(),audit_event_id uuid NOT NULL,
 FOREIGN KEY(workspace_id,association_id) REFERENCES crm.opportunity_contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.opportunity_contact_changes(workspace_id,association_id,occurred_at,id);
GRANT SELECT,INSERT,UPDATE ON crm.opportunity_contacts TO looski_runtime;
GRANT SELECT,INSERT ON crm.opportunity_contact_changes TO looski_runtime;
CREATE TABLE crm.opportunity_organizations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,
 opportunity_id uuid NOT NULL,organization_id uuid NOT NULL,
 role_label text NOT NULL DEFAULT '' CHECK(length(role_label)<=80),is_primary boolean NOT NULL DEFAULT false,
 ended_at timestamptz,created_by_membership_id uuid NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK(NOT is_primary OR ended_at IS NULL),UNIQUE(workspace_id,id),UNIQUE(workspace_id,opportunity_id,organization_id),
 FOREIGN KEY(workspace_id,opportunity_id) REFERENCES crm.opportunities(workspace_id,id),
 FOREIGN KEY(workspace_id,organization_id) REFERENCES contacts.organizations(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE UNIQUE INDEX ON crm.opportunity_organizations(workspace_id,opportunity_id) WHERE is_primary AND ended_at IS NULL;
CREATE INDEX ON crm.opportunity_organizations(workspace_id,opportunity_id,created_at,id);
CREATE INDEX ON crm.opportunity_organizations(workspace_id,organization_id);
CREATE TABLE crm.opportunity_organization_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,association_id uuid NOT NULL,
 role_label text NOT NULL,is_primary boolean NOT NULL,ended boolean NOT NULL,
 occurred_at timestamptz NOT NULL DEFAULT now(),audit_event_id uuid NOT NULL,
 FOREIGN KEY(workspace_id,association_id) REFERENCES crm.opportunity_organizations(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.opportunity_organization_changes(workspace_id,association_id,occurred_at,id);
GRANT SELECT,INSERT,UPDATE ON crm.opportunity_organizations TO looski_runtime;
GRANT SELECT,INSERT ON crm.opportunity_organization_changes TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES('crm.opportunities.participants.manage','Manage participants on permitted opportunities');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,'crm.opportunities.participants.manage' FROM suite.roles WHERE key='owner';
CREATE TABLE crm.deal_contacts (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,
 deal_id uuid NOT NULL,contact_id uuid NOT NULL,
 role_label text NOT NULL DEFAULT '' CHECK(length(role_label)<=80),is_primary boolean NOT NULL DEFAULT false,
 ended_at timestamptz,created_by_membership_id uuid NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK(NOT is_primary OR ended_at IS NULL),UNIQUE(workspace_id,id),UNIQUE(workspace_id,deal_id,contact_id),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,contact_id) REFERENCES contacts.contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE UNIQUE INDEX ON crm.deal_contacts(workspace_id,deal_id) WHERE is_primary AND ended_at IS NULL;
CREATE INDEX ON crm.deal_contacts(workspace_id,deal_id,created_at,id);
CREATE INDEX ON crm.deal_contacts(workspace_id,contact_id);
CREATE TABLE crm.deal_contact_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,association_id uuid NOT NULL,
 role_label text NOT NULL,is_primary boolean NOT NULL,ended boolean NOT NULL,
 occurred_at timestamptz NOT NULL DEFAULT now(),audit_event_id uuid NOT NULL,
 FOREIGN KEY(workspace_id,association_id) REFERENCES crm.deal_contacts(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.deal_contact_changes(workspace_id,association_id,occurred_at,id);
GRANT SELECT,INSERT,UPDATE ON crm.deal_contacts TO looski_runtime;
GRANT SELECT,INSERT ON crm.deal_contact_changes TO looski_runtime;
CREATE TABLE crm.deal_organizations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,
 deal_id uuid NOT NULL,organization_id uuid NOT NULL,
 role_label text NOT NULL DEFAULT '' CHECK(length(role_label)<=80),is_primary boolean NOT NULL DEFAULT false,
 ended_at timestamptz,created_by_membership_id uuid NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK(NOT is_primary OR ended_at IS NULL),UNIQUE(workspace_id,id),UNIQUE(workspace_id,deal_id,organization_id),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,organization_id) REFERENCES contacts.organizations(workspace_id,id),
 FOREIGN KEY(workspace_id,created_by_membership_id) REFERENCES suite.memberships(workspace_id,id)
);
CREATE UNIQUE INDEX ON crm.deal_organizations(workspace_id,deal_id) WHERE is_primary AND ended_at IS NULL;
CREATE INDEX ON crm.deal_organizations(workspace_id,deal_id,created_at,id);
CREATE INDEX ON crm.deal_organizations(workspace_id,organization_id);
CREATE TABLE crm.deal_organization_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,association_id uuid NOT NULL,
 role_label text NOT NULL,is_primary boolean NOT NULL,ended boolean NOT NULL,
 occurred_at timestamptz NOT NULL DEFAULT now(),audit_event_id uuid NOT NULL,
 FOREIGN KEY(workspace_id,association_id) REFERENCES crm.deal_organizations(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.deal_organization_changes(workspace_id,association_id,occurred_at,id);
GRANT SELECT,INSERT,UPDATE ON crm.deal_organizations TO looski_runtime;
GRANT SELECT,INSERT ON crm.deal_organization_changes TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES('crm.deals.participants.manage','Manage participants on permitted deals');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,'crm.deals.participants.manage' FROM suite.roles WHERE key='owner';
UPDATE suite.schema_version SET version=13;

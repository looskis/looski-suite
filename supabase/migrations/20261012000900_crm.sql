-- Optional CRM, workspace-owned definitions and separate sales/transaction records.
CREATE TABLE suite.workspace_modules (
 workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 module_key text NOT NULL CHECK(module_key IN ('crm')),
 enabled boolean NOT NULL DEFAULT false,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 updated_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,module_key)
);
GRANT SELECT,INSERT,UPDATE ON suite.workspace_modules TO looski_runtime;
CREATE SCHEMA crm;
REVOKE ALL ON SCHEMA crm FROM PUBLIC,anon,authenticated;
GRANT USAGE ON SCHEMA crm TO looski_runtime;
CREATE TABLE crm.opportunity_pipelines (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),position integer NOT NULL DEFAULT 0,
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),UNIQUE(workspace_id,id)
);
CREATE TABLE crm.opportunity_stages (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),pipeline_id uuid NOT NULL,
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),position integer NOT NULL DEFAULT 0,
 outcome text NOT NULL CHECK(outcome IN ('open','won','lost')),archived_at timestamptz,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,pipeline_id,id),FOREIGN KEY(workspace_id,pipeline_id) REFERENCES crm.opportunity_pipelines(workspace_id,id)
);
CREATE TABLE crm.deal_categories (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),position integer NOT NULL DEFAULT 0,
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),UNIQUE(workspace_id,id)
);
CREATE TABLE crm.deal_statuses (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),category_id uuid NOT NULL,
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),position integer NOT NULL DEFAULT 0,
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id),UNIQUE(workspace_id,category_id,id),FOREIGN KEY(workspace_id,category_id) REFERENCES crm.deal_categories(workspace_id,id)
);
CREATE TABLE crm.opportunities (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 200),notes text NOT NULL DEFAULT '' CHECK(length(notes)<=10000),
 owner_membership_id uuid NOT NULL,pipeline_id uuid NOT NULL,stage_id uuid NOT NULL,
 expected_value numeric(22,4) CHECK(expected_value>=0),currency_code text CHECK(currency_code ~ '^[A-Z]{3}$'),expected_close_on date,closed_at timestamptz,
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK((expected_value IS NULL)=(currency_code IS NULL)),UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,owner_membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,pipeline_id,stage_id) REFERENCES crm.opportunity_stages(workspace_id,pipeline_id,id)
);
CREATE TABLE crm.deals (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 200),notes text NOT NULL DEFAULT '' CHECK(length(notes)<=10000),
 owner_membership_id uuid NOT NULL,category_id uuid NOT NULL,status_id uuid NOT NULL,originating_opportunity_id uuid,
 value numeric(22,4) CHECK(value>=0),currency_code text CHECK(currency_code ~ '^[A-Z]{3}$'),starts_on date,ends_on date,
 archived_at timestamptz,version bigint NOT NULL DEFAULT 1 CHECK(version>0),created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK((value IS NULL)=(currency_code IS NULL)),CHECK(ends_on IS NULL OR starts_on IS NULL OR ends_on>=starts_on),UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,owner_membership_id) REFERENCES suite.memberships(workspace_id,id),
 FOREIGN KEY(workspace_id,category_id,status_id) REFERENCES crm.deal_statuses(workspace_id,category_id,id),
 FOREIGN KEY(workspace_id,originating_opportunity_id) REFERENCES crm.opportunities(workspace_id,id)
);
CREATE INDEX ON crm.opportunities(workspace_id,owner_membership_id,id);
CREATE INDEX ON crm.opportunities(workspace_id,pipeline_id,stage_id,id);
CREATE INDEX ON crm.deals(workspace_id,owner_membership_id,id);
CREATE INDEX ON crm.deals(workspace_id,originating_opportunity_id,id);
CREATE INDEX ON crm.deals(workspace_id,category_id,status_id,id);
ALTER TABLE suite.audit_events ADD UNIQUE(workspace_id,id);
CREATE TABLE crm.opportunity_stage_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,opportunity_id uuid NOT NULL,
 from_stage_id uuid,to_stage_id uuid NOT NULL,from_label text,to_label text NOT NULL,from_outcome text,to_outcome text NOT NULL,
 audit_event_id uuid NOT NULL,occurred_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,opportunity_id) REFERENCES crm.opportunities(workspace_id,id),
 FOREIGN KEY(workspace_id,from_stage_id) REFERENCES crm.opportunity_stages(workspace_id,id),
 FOREIGN KEY(workspace_id,to_stage_id) REFERENCES crm.opportunity_stages(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE TABLE crm.deal_status_changes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,deal_id uuid NOT NULL,
 from_status_id uuid,to_status_id uuid NOT NULL,from_label text,to_label text NOT NULL,
 audit_event_id uuid NOT NULL,occurred_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(workspace_id,deal_id) REFERENCES crm.deals(workspace_id,id),
 FOREIGN KEY(workspace_id,from_status_id) REFERENCES crm.deal_statuses(workspace_id,id),
 FOREIGN KEY(workspace_id,to_status_id) REFERENCES crm.deal_statuses(workspace_id,id),
 FOREIGN KEY(workspace_id,audit_event_id) REFERENCES suite.audit_events(workspace_id,id)
);
CREATE INDEX ON crm.opportunity_stage_changes(workspace_id,opportunity_id,occurred_at,id);
CREATE INDEX ON crm.deal_status_changes(workspace_id,deal_id,occurred_at,id);
GRANT SELECT,INSERT,UPDATE ON crm.opportunity_pipelines,crm.opportunity_stages,crm.deal_categories,crm.deal_statuses,crm.opportunities,crm.deals TO looski_runtime;
GRANT SELECT,INSERT ON crm.opportunity_stage_changes,crm.deal_status_changes TO looski_runtime;
INSERT INTO suite.permissions VALUES
 ('workspace.modules.read','Read optional application selection'),('workspace.modules.manage','Enable or disable available applications'),
 ('crm.configuration.read','Read CRM definitions'),('crm.configuration.manage','Create and edit CRM definitions'),
 ('crm.opportunities.read','Read permitted opportunities'),('crm.opportunities.create','Create opportunities'),('crm.opportunities.update','Edit permitted opportunities'),('crm.opportunities.archive','Archive or restore permitted opportunities'),
 ('crm.deals.read','Read permitted deals'),('crm.deals.create','Create deals'),('crm.deals.update','Edit permitted deals'),('crm.deals.archive','Archive or restore permitted deals');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT r.workspace_id,r.id,p.key FROM suite.roles r CROSS JOIN suite.permissions p
 WHERE r.key='owner' AND p.key IN ('workspace.modules.read','workspace.modules.manage','crm.configuration.read','crm.configuration.manage','crm.opportunities.read','crm.opportunities.create','crm.opportunities.update','crm.opportunities.archive','crm.deals.read','crm.deals.create','crm.deals.update','crm.deals.archive');
UPDATE suite.schema_version SET version=10;

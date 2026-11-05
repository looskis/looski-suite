-- Extensible record attributes share the record's authorization, version and lifecycle.
ALTER TABLE contacts.contacts ADD COLUMN properties jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(properties)='object' AND octet_length(properties::text)<=32768);
ALTER TABLE contacts.organizations ADD COLUMN properties jsonb NOT NULL DEFAULT '{}' CHECK(jsonb_typeof(properties)='object' AND octet_length(properties::text)<=32768);
ALTER TABLE contacts.contact_identifiers ADD COLUMN position integer NOT NULL DEFAULT 0 CHECK(position>=0);
CREATE INDEX contacts_properties_gin ON contacts.contacts USING gin(properties jsonb_path_ops);
CREATE INDEX organizations_properties_gin ON contacts.organizations USING gin(properties jsonb_path_ops);
CREATE INDEX contacts_status_idx ON contacts.contacts(workspace_id,(properties->>'status')) WHERE archived_at IS NULL AND merged_into_id IS NULL;
CREATE INDEX organizations_status_idx ON contacts.organizations(workspace_id,(properties->>'status')) WHERE archived_at IS NULL;
GRANT UPDATE(properties) ON contacts.contacts,contacts.organizations TO looski_runtime;
GRANT UPDATE(position) ON contacts.contact_identifiers TO looski_runtime;
CREATE TABLE suite.record_fields (
 workspace_id uuid NOT NULL REFERENCES suite.workspaces(id), kind text NOT NULL CHECK(kind IN ('contacts','organizations')),
 key text NOT NULL CHECK(key ~ '^custom_[a-z][a-z0-9_]{0,47}$'), name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 80),
 field_type text NOT NULL CHECK(field_type IN ('text','number','date','boolean','select','email','url')),
 options jsonb NOT NULL DEFAULT '[]' CHECK(jsonb_typeof(options)='array' AND jsonb_array_length(options)<=100),
 archived boolean NOT NULL DEFAULT false, version bigint NOT NULL DEFAULT 1, created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(workspace_id,kind,key)
);
CREATE TABLE suite.record_views (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL REFERENCES suite.workspaces(id),
 kind text NOT NULL CHECK(kind IN ('contacts','organizations')), owner_user_id uuid NOT NULL REFERENCES suite.users(id),
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 80), shared boolean NOT NULL DEFAULT false,
 config jsonb NOT NULL CHECK(jsonb_typeof(config)='object' AND octet_length(config::text)<=32768),
 version bigint NOT NULL DEFAULT 1, created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(workspace_id,id)
);
CREATE INDEX record_views_owner ON suite.record_views(workspace_id,kind,owner_user_id);
REVOKE ALL ON suite.record_fields,suite.record_views FROM PUBLIC,anon,authenticated;
GRANT SELECT,INSERT,UPDATE ON suite.record_fields,suite.record_views TO looski_runtime;
GRANT DELETE ON suite.record_views TO looski_runtime;
INSERT INTO suite.permissions(key,description) VALUES ('records.fields.manage','Manage workspace custom field definitions');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key) SELECT workspace_id,id,'records.fields.manage' FROM suite.roles WHERE key='owner';
ALTER TABLE crm.opportunities ADD COLUMN billing_frequency text NOT NULL DEFAULT 'one_time' CHECK(billing_frequency IN ('one_time','monthly','quarterly','annually'));
ALTER TABLE crm.deals ADD COLUMN billing_frequency text NOT NULL DEFAULT 'one_time' CHECK(billing_frequency IN ('one_time','monthly','quarterly','annually'));
GRANT UPDATE(billing_frequency) ON crm.opportunities,crm.deals TO looski_runtime;
UPDATE suite.schema_version SET version=38;

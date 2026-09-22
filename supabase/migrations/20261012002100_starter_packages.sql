-- Fixed, versioned starter manifests use typed references, never business copies.
CREATE TABLE crm.starter_installations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),workspace_id uuid NOT NULL,
 package_key text NOT NULL DEFAULT 'general-crm' CHECK(package_key='general-crm'),package_version integer NOT NULL CHECK(package_version=1),
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),installed_at timestamptz NOT NULL DEFAULT now(),removed_at timestamptz,
 installed_audit_id uuid NOT NULL,removed_audit_id uuid,removal_summary jsonb,
 pipeline_id uuid NOT NULL,category_id uuid NOT NULL,note_type_id uuid NOT NULL,interaction_type_id uuid NOT NULL,
 task_type_id uuid NOT NULL,task_template_id uuid NOT NULL,coordination_template_id uuid NOT NULL,rule_id uuid NOT NULL,
 UNIQUE(workspace_id,id),
 FOREIGN KEY(workspace_id,installed_audit_id) REFERENCES suite.audit_events(workspace_id,id),
 FOREIGN KEY(workspace_id,removed_audit_id) REFERENCES suite.audit_events(workspace_id,id),
 FOREIGN KEY(workspace_id,pipeline_id) REFERENCES crm.opportunity_pipelines(workspace_id,id),
 FOREIGN KEY(workspace_id,category_id) REFERENCES crm.deal_categories(workspace_id,id),
 FOREIGN KEY(workspace_id,note_type_id) REFERENCES crm.activity_types(workspace_id,id),
 FOREIGN KEY(workspace_id,interaction_type_id) REFERENCES crm.activity_types(workspace_id,id),
 FOREIGN KEY(workspace_id,task_type_id) REFERENCES tasks.task_types(workspace_id,id),
 FOREIGN KEY(workspace_id,task_template_id) REFERENCES tasks.templates(workspace_id,id),
 FOREIGN KEY(workspace_id,coordination_template_id) REFERENCES crm.coordination_templates(workspace_id,id),
 FOREIGN KEY(workspace_id,rule_id) REFERENCES crm.coordination_rules(workspace_id,id),
 CHECK((removed_at IS NULL)=(removed_audit_id IS NULL)),CHECK(removal_summary IS NULL OR octet_length(removal_summary::text)<=65536)
);
CREATE UNIQUE INDEX ON crm.starter_installations(workspace_id,package_key) WHERE removed_at IS NULL;
CREATE INDEX ON crm.starter_installations(workspace_id,installed_at,id);
CREATE TABLE crm.starter_stages (
 workspace_id uuid NOT NULL,installation_id uuid NOT NULL,stage_id uuid NOT NULL,position integer NOT NULL CHECK(position BETWEEN 0 AND 4),
 PRIMARY KEY(workspace_id,installation_id,stage_id),UNIQUE(workspace_id,installation_id,position),
 FOREIGN KEY(workspace_id,installation_id) REFERENCES crm.starter_installations(workspace_id,id),
 FOREIGN KEY(workspace_id,stage_id) REFERENCES crm.opportunity_stages(workspace_id,id)
);
CREATE TABLE crm.starter_statuses (
 workspace_id uuid NOT NULL,installation_id uuid NOT NULL,status_id uuid NOT NULL,position integer NOT NULL CHECK(position BETWEEN 0 AND 2),
 PRIMARY KEY(workspace_id,installation_id,status_id),UNIQUE(workspace_id,installation_id,position),
 FOREIGN KEY(workspace_id,installation_id) REFERENCES crm.starter_installations(workspace_id,id),
 FOREIGN KEY(workspace_id,status_id) REFERENCES crm.deal_statuses(workspace_id,id)
);
GRANT SELECT,INSERT,UPDATE ON crm.starter_installations TO looski_runtime;
GRANT SELECT,INSERT ON crm.starter_stages,crm.starter_statuses TO looski_runtime;
UPDATE suite.schema_version SET version=22;

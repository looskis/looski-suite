-- Export capability never expands independent per-resource read visibility.
INSERT INTO suite.permissions(key,description) VALUES ('workspace.data.export','Export permitted business records through the documented JSON contract');
INSERT INTO suite.role_permissions(workspace_id,role_id,permission_key)
 SELECT workspace_id,id,'workspace.data.export' FROM suite.roles WHERE key='owner';
UPDATE suite.schema_version SET version=27;

-- Timeline sources remain owned by their original modules. No duplicated state.
CREATE INDEX ON tasks.task_changes(workspace_id,occurred_at DESC,id DESC);
UPDATE suite.schema_version SET version=23;

import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { taskFields, taskDraft, taskInput } from '$lib/server/task-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, id: event.params.task };
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/tasks/{id}',
    { params: { path } },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Task unavailable');
  const history = await client.GET(
    '/api/v1/workspaces/{workspace_id}/tasks/{id}/history',
    {
      params: {
        path,
        query: {
          limit: 50,
          after: event.url.searchParams.get('history_after') ?? undefined,
        },
      },
    },
  );
  if (!history.data)
    error(
      history.response.status,
      history.error?.message ?? 'History unavailable',
    );
  return {
    workspace,
    task: result.data,
    fields: taskFields(result.data),
    history: history.data,
    key: randomUUID(),
    archiveKey: randomUUID(),
  };
};
export const actions: Actions = {
  save: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const fields = taskInput(form);
    const version = Number(form.get('expected_version'));
    if (!fields || !Number.isSafeInteger(version) || version < 1)
      return fail(400, {
        message: 'Valid status and version required',
        draft: taskDraft(form),
      });
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/tasks/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.task },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: { ...fields, expected_version: version },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save task',
        draft: taskDraft(form),
      });
    redirect(303, event.url.pathname);
  },
  archive: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const archived = String(form.get('archived'));
    const version = Number(form.get('expected_version'));
    const archiveDraft = {
      archived,
      expected_version: String(form.get('expected_version') ?? ''),
      key: String(form.get('idempotency_key') ?? ''),
    };
    if (
      !['true', 'false'].includes(archived) ||
      !Number.isSafeInteger(version) ||
      version < 1
    )
      return fail(400, {
        message: 'Valid archive state and version required',
        archiveDraft,
      });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/tasks/{id}/archive',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.task },
          header: { 'Idempotency-Key': archiveDraft.key },
        },
        body: { archived: archived === 'true', expected_version: version },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change archive state',
        archiveDraft,
      });
    redirect(303, event.url.pathname);
  },
};

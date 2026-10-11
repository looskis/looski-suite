import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { dateOrNull, retryKey, text, version } from '$lib/server/project-form';
import type { components } from '$lib/api/schema';
import type { Actions, PageServerLoad, RequestEvent } from './$types';
type TaskStatus = components['schemas']['Status'];
type ProjectStatus = components['schemas']['ProjectStatus'];
const taskStatus = (form: FormData) =>
  (String(form.get('task_status') ?? '') || null) as TaskStatus | null;
async function session(event: RequestEvent) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, project_id: event.params.project };
  return { client, path, form: await event.request.formData() };
}
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, project_id: event.params.project };
  const project = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/{project_id}',
    { params: { path } },
  );
  if (!project.data)
    error(
      project.response.status,
      project.error?.message ?? 'Project unavailable',
    );
  const columns = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/{project_id}/columns',
    { params: { path } },
  );
  return {
    workspace,
    project: project.data,
    columns: columns.data ?? [],
    key: randomUUID(),
  };
};
export const actions: Actions = {
  save: async (event) => {
    const { client, path, form } = await session(event);
    const expected = version(form);
    const input = {
      name: text(form, 'name'),
      description: text(form, 'description'),
      status: String(form.get('status') ?? 'active') as ProjectStatus,
      due_on: dateOrNull(form, 'due_on'),
    };
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}',
      {
        params: { path, header: { 'Idempotency-Key': retryKey(form) } },
        body: { ...input, expected_version: expected },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save the project',
        input,
      });
    redirect(303, event.url.pathname);
  },
  archive: async (event) => {
    const { client, path, form } = await session(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/archive',
      {
        params: { path, header: { 'Idempotency-Key': retryKey(form) } },
        body: {
          archived: form.get('archived') === 'true',
          expected_version: version(form) ?? 0,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change archive state',
      });
    redirect(303, event.url.pathname);
  },
  addColumn: async (event) => {
    const { client, path, form } = await session(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/columns',
      {
        params: { path, header: { 'Idempotency-Key': retryKey(form) } },
        body: { name: text(form, 'name'), task_status: taskStatus(form) },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not add the column',
      });
    redirect(303, event.url.pathname);
  },
  saveColumn: async (event) => {
    const { client, path, form } = await session(event);
    const position = String(form.get('position') ?? '');
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/columns/{column_id}',
      {
        params: {
          path: { ...path, column_id: String(form.get('column_id') ?? '') },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: {
          name: text(form, 'name'),
          task_status: taskStatus(form),
          position: position ? Number(position) - 1 : null,
          expected_version: version(form),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save the column',
      });
    redirect(303, event.url.pathname);
  },
  removeColumn: async (event) => {
    const { client, path, form } = await session(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/columns/{column_id}/remove',
      {
        params: {
          path: { ...path, column_id: String(form.get('column_id') ?? '') },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: {
          expected_version: version(form) ?? 0,
          move_cards_to: String(form.get('move_cards_to') ?? ''),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not remove the column',
      });
    redirect(303, event.url.pathname);
  },
};

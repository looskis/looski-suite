import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { dateOrNull, retryKey, text, version } from '$lib/server/project-form';
import type { components } from '$lib/api/schema';
import type { Actions, PageServerLoad, RequestEvent } from './$types';
type TaskStatus = components['schemas']['Status'];
async function session(event: RequestEvent) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  return {
    client,
    workspace,
    form: await event.request.formData(),
    path: {
      workspace_id: workspace.id,
      project_id: event.params.project,
    },
  };
}
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, project_id: event.params.project };
  const cardPath = { ...path, card_id: event.params.card };
  const card = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}',
    { params: { path: cardPath } },
  );
  if (!card.data)
    error(card.response.status, card.error?.message ?? 'Task unavailable');
  const [project, members, comments] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/projects/{project_id}', {
      params: { path },
    }),
    client.GET(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/members',
      { params: { path } },
    ),
    client.GET(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/comments',
      {
        params: {
          path: cardPath,
          query: {
            limit: 50,
            after: event.url.searchParams.get('after') ?? undefined,
          },
        },
      },
    ),
  ]);
  if (!comments.data)
    error(
      comments.response.status,
      comments.error?.message ?? 'Comments unavailable',
    );
  return {
    workspace,
    card: card.data,
    project: project.data,
    members: members.data ?? [],
    comments: comments.data,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  schedule: async (event) => {
    const { client, path, form } = await session(event);
    const status = String(form.get('status') ?? '') as TaskStatus;
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/schedule',
      {
        params: {
          path: { ...path, card_id: event.params.card },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: {
          expected_task_version: version(form) ?? 0,
          assigned_membership_id: String(
            form.get('assigned_membership_id') ?? '',
          ),
          due_on: dateOrNull(form, 'due_on'),
          status,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save the plan',
      });
    redirect(303, event.url.pathname);
  },
  comment: async (event) => {
    const { client, path, form } = await session(event);
    const body = text(form, 'body');
    const key = retryKey(form);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/comments',
      {
        params: {
          path: { ...path, card_id: event.params.card },
          header: { 'Idempotency-Key': key },
        },
        body: { body },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        commentMessage: result.error?.message ?? 'Could not add the comment',
        body,
        key,
      });
    redirect(303, event.url.pathname);
  },
  edit: async (event) => {
    const { client, path, form } = await session(event);
    const comment = String(form.get('comment_id') ?? '');
    const body = text(form, 'body');
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/comments/{comment_id}',
      {
        params: {
          path: { ...path, comment_id: comment },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: { body, expected_version: version(form) ?? 0 },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        commentMessage: result.error?.message ?? 'Could not edit the comment',
        editing: comment,
        editBody: body,
      });
    redirect(303, event.url.pathname);
  },
  remove: async (event) => {
    const { client, path, form } = await session(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/comments/{comment_id}/remove',
      {
        params: {
          path: { ...path, comment_id: String(form.get('comment_id') ?? '') },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: { expected_version: version(form) ?? 0 },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        commentMessage: result.error?.message ?? 'Could not remove the comment',
      });
    redirect(303, event.url.pathname);
  },
  removeCard: async (event) => {
    const { client, path, form, workspace } = await session(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/remove',
      {
        params: {
          path: { ...path, card_id: event.params.card },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: { expected_version: version(form) ?? 0 },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not take it off the board',
      });
    redirect(303, `/workspaces/${workspace.id}/projects/${path.project_id}`);
  },
};

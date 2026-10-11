import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { dateOrNull, retryKey, text, version } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, project_id: event.params.project };
  const board = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/{project_id}/board',
    { params: { path } },
  );
  if (!board.data)
    error(board.response.status, board.error?.message ?? 'Board unavailable');
  const members = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/{project_id}/members',
    { params: { path } },
  );
  // Existing tasks to add come from Tasks' own read, which may be narrower or denied.
  const tasks = await client.GET('/api/v1/workspaces/{workspace_id}/tasks', {
    params: {
      path: { workspace_id: workspace.id },
      query: { limit: 100, archived: false },
    },
  });
  const onBoard = new Set(
    board.data.columns.flatMap((c) => c.cards.map((card) => card.task.id)),
  );
  return {
    workspace,
    board: board.data,
    members: members.data ?? [],
    tasks: (tasks.data?.items ?? []).filter((t) => !onBoard.has(t.id)),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  add: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const assignee = String(form.get('assigned_membership_id') ?? '');
    const input = {
      title: text(form, 'title'),
      due_on: dateOrNull(form, 'due_on'),
      assigned_membership_id: assignee || null,
    };
    const key = retryKey(form);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            project_id: event.params.project,
          },
          header: { 'Idempotency-Key': key },
        },
        body: {
          column_id: String(form.get('column_id') ?? '') || null,
          task: input,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not add the task',
        input,
        key,
      });
    redirect(303, event.url.pathname);
  },
  link: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            project_id: event.params.project,
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: { task_id: String(form.get('task_id') ?? '') },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not add the task',
      });
    redirect(303, event.url.pathname);
  },
  move: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const expected = version(form);
    if (!expected) return fail(400, { message: 'Reload the board' });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/cards/{card_id}/move',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            project_id: event.params.project,
            card_id: String(form.get('card_id') ?? ''),
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: {
          expected_version: expected,
          column_id: String(form.get('column_id') ?? ''),
          before_card_id: String(form.get('before_card_id') ?? '') || null,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not move the card',
      });
    redirect(303, event.url.pathname);
  },
};

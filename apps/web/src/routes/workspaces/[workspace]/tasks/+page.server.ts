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
  const search = event.url.searchParams.get('search') ?? '';
  const archived = event.url.searchParams.get('archived') === 'true';
  const status = event.url.searchParams.get('status') || undefined;
  if (
    status !== undefined &&
    status !== 'open' &&
    status !== 'in_progress' &&
    status !== 'completed' &&
    status !== 'cancelled'
  )
    error(400, 'Unsupported status');
  const assignedMembershipId =
    event.url.searchParams.get('assigned_membership_id') ?? '';
  const dueBefore = event.url.searchParams.get('due_before') ?? '';
  const result = await client.GET('/api/v1/workspaces/{workspace_id}/tasks', {
    params: {
      path: { workspace_id: workspace.id },
      query: {
        limit: 50,
        after: event.url.searchParams.get('after') ?? undefined,
        search,
        archived,
        status,
        due_before: dueBefore || undefined,
        assigned_membership_id: assignedMembershipId || undefined,
      },
    },
  });
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Tasks unavailable');
  return {
    workspace,
    page: result.data,
    search,
    archived,
    status: status ?? '',
    dueBefore,
    assignedMembershipId,
    fields: taskFields(),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const body = taskInput(form);
    if (!body)
      return fail(400, {
        message: 'Unsupported status',
        draft: taskDraft(form),
      });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/tasks',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create task',
        draft: taskDraft(form),
      });
    redirect(303, event.url.pathname + '/' + result.data.id);
  },
};

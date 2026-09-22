import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { typeDraft, typeInput } from '$lib/server/task-type-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/tasks/types/{id}',
    {
      params: {
        path: { workspace_id: workspace.id, id: event.params.definition },
      },
    },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Type unavailable');
  return { workspace, type: result.data, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const draft = typeDraft(form);
    const input = typeInput(form, true);
    if (!input)
      return fail(400, {
        message: 'Valid order, availability and version required',
        draft,
      });
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/tasks/types/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.definition },
          header: { 'Idempotency-Key': draft.key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not update type',
        draft,
      });
    redirect(303, event.url.pathname);
  },
};

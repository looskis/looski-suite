import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  templateFields,
  templateDraft,
  templateInput,
} from '$lib/server/task-template-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const archived = event.url.searchParams.get('archived') ?? 'false';
  if (!['true', 'false'].includes(archived))
    error(400, 'Invalid archive filter');
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/tasks/templates',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          limit: 50,
          after: event.url.searchParams.get('after') ?? undefined,
          archived: archived === 'true',
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Templates unavailable',
    );
  return {
    workspace,
    page: result.data,
    archived: archived === 'true',
    fields: templateFields(),
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
    const draft = templateDraft(form);
    const input = templateInput(form, false);
    if (!input)
      return fail(400, {
        message: 'Valid assignment, offset and availability required',
        draft,
      });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/tasks/templates',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': draft.key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create template',
        draft,
      });
    redirect(303, `${event.url.pathname}/${result.data.id}`);
  },
};

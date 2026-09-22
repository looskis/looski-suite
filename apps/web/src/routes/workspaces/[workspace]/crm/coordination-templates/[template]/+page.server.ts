import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  coordinationFields,
  coordinationDraft,
  coordinationInput,
} from '$lib/server/coordination-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, id: event.params.template };
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/coordination-templates/{id}',
    { params: { path } },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Template unavailable',
    );
  const revisions = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/coordination-templates/{id}/revisions',
    {
      params: {
        path,
        query: {
          limit: 25,
          after: event.url.searchParams.get('after') ?? undefined,
        },
      },
    },
  );
  if (!revisions.data)
    error(
      revisions.response.status,
      revisions.error?.message ?? 'Revisions unavailable',
    );
  return {
    workspace,
    template: result.data,
    revisions: revisions.data,
    fields: coordinationFields(result.data),
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
    const draft = coordinationDraft(form);
    const input = coordinationInput(form, true);
    if (!input)
      return fail(400, {
        message: 'Provide 1–25 checklist items, valid availability and version',
        draft,
      });
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/crm/coordination-templates/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.template },
          header: { 'Idempotency-Key': draft.key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not update template',
        draft,
      });
    redirect(303, event.url.pathname);
  },
};

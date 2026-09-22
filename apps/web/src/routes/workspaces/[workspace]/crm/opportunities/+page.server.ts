import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  workflowChoices,
  recordFields,
  recordDraft,
  opportunityInput,
} from '$lib/server/crm-record-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const search = event.url.searchParams.get('search') ?? '';
  const tagId = event.url.searchParams.get('tag_id') ?? '';
  const archived = event.url.searchParams.get('archived') === 'true';
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/opportunities',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          limit: 50,
          after: event.url.searchParams.get('after') ?? undefined,
          search,
          tag_id: tagId || undefined,
          archived,
        },
      },
    },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'CRM unavailable');
  const workflow = await workflowChoices(
    client,
    workspace.id,
    false,
    event.url.searchParams.get('parent'),
    undefined,
    event.url.searchParams.get('states_after'),
  );
  const origins: { id: string; name: string }[] = [];
  const origin = '';
  const catalog = await client.GET('/api/v1/workspaces/{workspace_id}/tags', {
    params: { path: { workspace_id: workspace.id }, query: { limit: 100 } },
  });
  if (!catalog.data && catalog.response.status !== 403)
    error(
      catalog.response.status,
      catalog.error?.message ?? 'Tags unavailable',
    );
  return {
    tagChoices: catalog.data?.items ?? [],
    workspace,
    page: result.data,
    workflow,
    fields: recordFields(),
    search,
    tagId,
    archived,
    origins,
    origin,
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
    const input = opportunityInput(form);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/crm/opportunities',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: { ...input },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create opportunity',
        draft: recordDraft(form),
      });
    redirect(303, event.url.pathname + '/' + result.data.id);
  },
};

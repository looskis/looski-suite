import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  workflowChoices,
  recordFields,
  recordDraft,
  dealInput,
} from '$lib/server/crm-record-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const search = event.url.searchParams.get('search') ?? '';
  const archived = event.url.searchParams.get('archived') === 'true';
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/deals',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          limit: 50,
          after: event.url.searchParams.get('after') ?? undefined,
          search,
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
    true,
    event.url.searchParams.get('parent'),
    undefined,
    event.url.searchParams.get('states_after'),
  );
  let origins: { id: string; name: string }[] = [];
  const resultOrigins = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/opportunities',
    { params: { path: { workspace_id: workspace.id }, query: { limit: 100 } } },
  );
  if (resultOrigins.data)
    origins = resultOrigins.data.items.map((o) => ({
      id: o.id,
      name: o.title,
    }));
  else if (resultOrigins.response.status !== 403)
    error(
      resultOrigins.response.status,
      resultOrigins.error?.message ?? 'Opportunities unavailable',
    );
  const origin = event.url.searchParams.get('origin') ?? '';
  if (origin && !origins.some((o) => o.id === origin)) {
    const item = await client.GET(
      '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}',
      { params: { path: { workspace_id: workspace.id, id: origin } } },
    );
    if (!item.data)
      error(
        item.response.status,
        item.error?.message ?? 'Opportunity unavailable',
      );
    origins.push({ id: item.data.id, name: item.data.title });
  }
  return {
    workspace,
    page: result.data,
    workflow,
    fields: recordFields(),
    search,
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
    const input = dealInput(form);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/crm/deals',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          ...input,
          originating_opportunity_id: String(form.get('origin') ?? '') || null,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create deal',
        draft: recordDraft(form),
      });
    redirect(303, event.url.pathname + '/' + result.data.id);
  },
};

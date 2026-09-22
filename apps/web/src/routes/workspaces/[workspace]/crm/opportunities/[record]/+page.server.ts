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
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}',
    {
      params: { path: { workspace_id: workspace.id, id: event.params.record } },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Opportunity unavailable',
    );
  const history = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/history',
    {
      params: {
        path: { workspace_id: workspace.id, id: event.params.record },
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
  const workflow = await workflowChoices(
    client,
    workspace.id,
    false,
    event.url.searchParams.get('parent'),
    result.data,
    event.url.searchParams.get('states_after'),
  );
  return {
    workspace,
    record: result.data,
    workflow,
    fields: recordFields(result.data),
    history: history.data,
    key: randomUUID(),
    archive_key: randomUUID(),
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
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.record },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          ...opportunityInput(form),
          expected_version: Number(form.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save opportunity',
        draft: recordDraft(form),
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
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/archive',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.record },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          archived: form.get('archived') === 'true',
          expected_version: Number(form.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change archive state',
      });
    redirect(303, event.url.pathname);
  },
};

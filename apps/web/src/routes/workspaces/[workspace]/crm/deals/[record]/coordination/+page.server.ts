import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, id: event.params.record };
  const deal = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/deals/{id}',
    { params: { path } },
  );
  if (!deal.data)
    error(deal.response.status, deal.error?.message ?? 'Deal unavailable');
  const instances = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/coordination',
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
  if (!instances.data)
    error(
      instances.response.status,
      instances.error?.message ?? 'Coordination unavailable',
    );
  return {
    workspace,
    deal: deal.data,
    page: instances.data,
    key: randomUUID(),
    today: new Date().toISOString().slice(0, 10),
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
    const t = (k: string) => String(form.get(k) ?? '');
    const draft = {
      revision_id: t('revision_id'),
      base_on: t('base_on'),
      expected_version: t('expected_version'),
      key: t('idempotency_key'),
    };
    const version = Number(draft.expected_version);
    if (!Number.isSafeInteger(version) || version < 1)
      return fail(400, { message: 'Valid deal version required', draft });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/coordination',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.record },
          header: { 'Idempotency-Key': draft.key },
        },
        body: {
          revision_id: draft.revision_id,
          base_on: draft.base_on,
          expected_version: version,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not apply coordination',
        draft,
      });
    redirect(303, `${event.url.pathname}/${result.data.id}`);
  },
};

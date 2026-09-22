import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const r = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/starter-packages',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          limit: 25,
          after: event.url.searchParams.get('after') ?? undefined,
        },
      },
    },
  );
  if (!r.data)
    error(r.response.status, r.error?.message ?? 'Starter package unavailable');
  return { workspace, page: r.data, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const draft = {
      key: String(f.get('idempotency_key') ?? ''),
      package_version: String(f.get('package_version') ?? ''),
    };
    if (draft.package_version !== '1')
      return fail(400, {
        message: 'Choose an available package version',
        draft,
      });
    const r = await client.POST(
      '/api/v1/workspaces/{workspace_id}/crm/starter-packages',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': draft.key },
        },
        body: { package_version: 1 },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not install package',
        draft,
      });
    redirect(303, `${event.url.pathname}/${r.data.id}`);
  },
};

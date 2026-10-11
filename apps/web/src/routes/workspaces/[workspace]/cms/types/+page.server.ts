import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const archived = event.url.searchParams.get('archived') === 'true';
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/cms/types',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          archived,
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Content unavailable',
    );
  return { workspace, page: result.data, archived, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const name = String(form.get('name') ?? '');
    const description = String(form.get('description') ?? '').replaceAll(
      '\r\n',
      '\n',
    );
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/cms/types',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': key },
        },
        body: { name, description, fields: [] },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create this content type',
        input: { name, description },
        key,
      });
    redirect(303, `/workspaces/${workspace.id}/cms/types/${result.data.id}`);
  },
};

import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const search = event.url.searchParams.get('search') ?? '';
  const archived = event.url.searchParams.get('archived') === 'true';
  const result = await client.GET('/api/v1/workspaces/{workspace_id}/tags', {
    params: {
      path: { workspace_id: workspace.id },
      query: {
        search,
        archived,
        after: event.url.searchParams.get('after') ?? undefined,
        limit: 50,
      },
    },
  });
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Tags unavailable');
  return { workspace, page: result.data, search, archived, key: randomUUID() };
};
export const actions: Actions = {
  create: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const input = {
      name: String(form.get('name') ?? ''),
      color: String(form.get('color') ?? '') || null,
    };
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.POST('/api/v1/workspaces/{workspace_id}/tags', {
      params: {
        path: { workspace_id: workspace.id },
        header: { 'Idempotency-Key': key },
      },
      body: input,
    });
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create tag',
        input,
        key,
      });
    redirect(303, `/workspaces/${workspace.id}/tags/${result.data.id}`);
  },
};

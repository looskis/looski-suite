import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/tags/{tag_id}',
    {
      params: {
        path: { workspace_id: workspace.id, tag_id: event.params.tag },
      },
    },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Tag unavailable');
  return { workspace, tag: result.data, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const input = {
      name: String(form.get('name') ?? ''),
      color: String(form.get('color') ?? '') || null,
      archived: form.get('archived') === 'on',
      expected_version: Number(form.get('expected_version')),
    };
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/tags/{tag_id}',
      {
        params: {
          path: { workspace_id: workspace.id, tag_id: event.params.tag },
          header: { 'Idempotency-Key': key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save tag',
        input,
        key,
      });
    redirect(303, event.url.pathname);
  },
};

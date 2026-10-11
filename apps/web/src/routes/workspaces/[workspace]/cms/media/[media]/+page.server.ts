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
    '/api/v1/workspaces/{workspace_id}/cms/media/{media_id}',
    {
      params: {
        path: { workspace_id: workspace.id, media_id: event.params.media },
      },
    },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Media unavailable');
  return { workspace, item: result.data, key: randomUUID() };
};
export const actions: Actions = {
  save: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/cms/media/{media_id}',
      {
        params: {
          path: { workspace_id: workspace.id, media_id: event.params.media },
          header: { 'Idempotency-Key': key },
        },
        body: {
          expected_version: Number(form.get('expected_version')),
          title: String(form.get('title') ?? ''),
          alt_text: String(form.get('alt_text') ?? ''),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save this media',
        key,
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
      '/api/v1/workspaces/{workspace_id}/cms/media/{media_id}/archive',
      {
        params: {
          path: { workspace_id: workspace.id, media_id: event.params.media },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          expected_version: Number(form.get('expected_version')),
          archived: form.get('archived') === 'true',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change this media',
      });
    redirect(303, event.url.pathname);
  },
};

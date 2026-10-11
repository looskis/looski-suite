import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { applyOperation, fieldKinds, readFields } from '$lib/server/cms-form';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/cms/types/{type_id}',
    {
      params: {
        path: { workspace_id: workspace.id, type_id: event.params.type },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Content type unavailable',
    );
  return {
    workspace,
    item: result.data,
    kinds: fieldKinds,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  // Every builder button submits the whole form, so no edit is ever lost.
  save: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const key = String(form.get('idempotency_key') ?? '');
    const name = String(form.get('name') ?? '');
    const description = String(form.get('description') ?? '').replaceAll(
      '\r\n',
      '\n',
    );
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/cms/types/{type_id}',
      {
        params: {
          path: { workspace_id: workspace.id, type_id: event.params.type },
          header: { 'Idempotency-Key': key },
        },
        body: {
          name,
          description,
          fields: applyOperation(readFields(form), form),
          expected_version: Number(form.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save this content type',
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
      '/api/v1/workspaces/{workspace_id}/cms/types/{type_id}/archive',
      {
        params: {
          path: { workspace_id: workspace.id, type_id: event.params.type },
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
        message: result.error?.message ?? 'Could not change this content type',
      });
    redirect(303, event.url.pathname);
  },
};

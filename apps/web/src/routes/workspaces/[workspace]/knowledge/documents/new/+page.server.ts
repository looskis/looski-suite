import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { activeCollections } from '$lib/server/knowledge-collections';
import { retryKey, text } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const collections = await activeCollections(client, workspace.id);
  if (collections.error)
    error(
      collections.error.response.status,
      collections.error.error?.message ?? 'Knowledge base unavailable',
    );
  return {
    workspace,
    collections: collections.items,
    selected: event.url.searchParams.get('collection') ?? '',
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
    const input = {
      collection_id: String(form.get('collection_id') ?? ''),
      kind: (form.get('kind') === 'sop' ? 'sop' : 'article') as
        'sop' | 'article',
      title: text(form, 'title'),
      body: text(form, 'body'),
    };
    const key = retryKey(form);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/knowledge/documents',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create the document',
        input,
        key,
      });
    redirect(
      303,
      `/workspaces/${workspace.id}/knowledge/documents/${result.data.id}`,
    );
  },
};

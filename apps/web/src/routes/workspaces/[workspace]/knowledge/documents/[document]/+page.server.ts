import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { activeCollections } from '$lib/server/knowledge-collections';
import { retryKey, version } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const document = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}',
    {
      params: {
        path: {
          workspace_id: workspace.id,
          document_id: event.params.document,
        },
      },
    },
  );
  if (!document.data)
    error(
      document.response.status,
      document.error?.message ?? 'Document unavailable',
    );
  const collections = await activeCollections(client, workspace.id);
  return {
    workspace,
    document: document.data,
    targets: collections.items.filter(
      (c) => c.id !== document.data.collection_id,
    ),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  move: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/move',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            document_id: event.params.document,
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: {
          expected_version: version(form) ?? 0,
          collection_id: String(form.get('collection_id') ?? ''),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not move the document',
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
      '/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/archive',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            document_id: event.params.document,
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: {
          expected_version: version(form) ?? 0,
          archived: form.get('archived') === 'true',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change the document',
      });
    redirect(303, event.url.pathname);
  },
};

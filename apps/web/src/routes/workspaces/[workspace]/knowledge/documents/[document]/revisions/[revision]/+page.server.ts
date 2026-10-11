import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey, version } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = {
    workspace_id: workspace.id,
    document_id: event.params.document,
  };
  const document = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}',
    { params: { path } },
  );
  if (!document.data)
    error(
      document.response.status,
      document.error?.message ?? 'Document unavailable',
    );
  const revision = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/revisions/{revision_id}',
    { params: { path: { ...path, revision_id: event.params.revision } } },
  );
  if (!revision.data)
    error(
      revision.response.status,
      revision.error?.message ?? 'Revision unavailable',
    );
  return {
    workspace,
    document: document.data,
    revision: revision.data,
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
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}/revisions/{revision_id}/restore',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            document_id: event.params.document,
            revision_id: event.params.revision,
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: { expected_version: version(form) ?? 0 },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not restore this revision',
      });
    redirect(
      303,
      `/workspaces/${workspace.id}/knowledge/documents/${event.params.document}`,
    );
  },
};

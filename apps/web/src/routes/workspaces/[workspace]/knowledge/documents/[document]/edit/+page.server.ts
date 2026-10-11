import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey, text, version } from '$lib/server/project-form';
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
  return { workspace, document: document.data, key: randomUUID() };
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
      expected_version: version(form) ?? 0,
      kind: (form.get('kind') === 'sop' ? 'sop' : 'article') as
        'sop' | 'article',
      title: text(form, 'title'),
      body: text(form, 'body'),
    };
    const key = retryKey(form);
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/knowledge/documents/{document_id}',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            document_id: event.params.document,
          },
          header: { 'Idempotency-Key': key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        // A conflict keeps the draft: the form below still holds what was typed,
        // and reloading in another tab shows the newer text to reconcile.
        message: result.error?.message ?? 'Could not save the document',
        input,
        key,
      });
    redirect(
      303,
      `/workspaces/${workspace.id}/knowledge/documents/${event.params.document}`,
    );
  },
};

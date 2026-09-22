import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { fileDraft } from '$lib/server/file-form';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/files/{id}',
    { params: { path: { workspace_id: workspace.id, id: event.params.file } } },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'File unavailable');
  return { workspace, file: result.data, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const draft = fileDraft(form);
    const expected_version = Number(form.get('expected_version'));
    if (!Number.isSafeInteger(expected_version) || expected_version < 1)
      return fail(400, { draft, message: 'Reload the file before editing' });
    const params = {
      path: { workspace_id: workspace.id, id: event.params.file },
      header: { 'Idempotency-Key': String(form.get('idempotency_key') ?? '') },
    };
    const intent = form.get('intent');
    if (intent !== 'edit' && intent !== 'archive' && intent !== 'restore')
      return fail(400, { draft, message: 'Choose a supported file action' });
    const result =
      intent === 'edit'
        ? await client
            .PATCH('/api/v1/workspaces/{workspace_id}/files/{id}', {
              params,
              body: {
                expected_version,
                filename: String(form.get('filename') ?? ''),
                media_type: String(form.get('media_type') ?? ''),
                notes: String(form.get('notes') ?? ''),
                owner_membership_id:
                  String(form.get('owner_membership_id') ?? '') || null,
              },
            })
            .catch(() => null)
        : await client
            .POST('/api/v1/workspaces/{workspace_id}/files/{id}/archive', {
              params,
              body: { expected_version, archived: intent === 'archive' },
            })
            .catch(() => null);
    if (!result?.data)
      return fail(result?.response.status ?? 503, {
        draft,
        message:
          result?.error?.message ?? 'Result unavailable; retry the same change',
      });
    redirect(303, event.url.pathname);
  },
};

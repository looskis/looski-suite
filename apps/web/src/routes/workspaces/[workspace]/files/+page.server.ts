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
  const archived = event.url.searchParams.get('archived') === 'true';
  const search = event.url.searchParams.get('search') ?? '';
  const result = await client.GET('/api/v1/workspaces/{workspace_id}/files', {
    params: {
      path: { workspace_id: workspace.id },
      query: {
        archived,
        search,
        after: event.url.searchParams.get('after') ?? undefined,
        limit: 50,
      },
    },
  });
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Files unavailable');
  const nextQuery = new URLSearchParams({ search, archived: String(archived) });
  if (result.data.next_cursor) nextQuery.set('after', result.data.next_cursor);
  return {
    workspace,
    page: result.data,
    archived,
    search,
    next: result.data.next_cursor ? '?' + nextQuery : null,
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
    const draft = fileDraft(form);
    const file = form.get('file');
    if (!file || typeof file === 'string' || !file.name)
      return fail(400, { draft, message: 'Choose a file to upload' });
    if (file.size > 8 * 1024 * 1024)
      return fail(400, { draft, message: 'Files must be 8 MiB or smaller' });
    const result = await client
      .POST('/api/v1/workspaces/{workspace_id}/files', {
        params: {
          path: { workspace_id: workspace.id },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          filename: file.name,
          media_type: file.type || 'application/octet-stream',
          notes: String(form.get('notes') ?? ''),
          owner_membership_id:
            String(form.get('owner_membership_id') ?? '') || null,
          content_base64: Buffer.from(await file.arrayBuffer()).toString(
            'base64',
          ),
        },
      })
      .catch(() => null);
    if (!result?.data)
      return fail(result?.response.status ?? 503, {
        draft,
        message:
          result?.error?.message ??
          'Upload result unavailable; retry with the same file',
      });
    redirect(303, event.url.pathname + '/' + result.data.id);
  },
};

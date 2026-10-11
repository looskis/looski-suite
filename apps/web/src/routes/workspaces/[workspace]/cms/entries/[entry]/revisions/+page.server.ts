import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, entry_id: event.params.entry };
  const [entry, revisions] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}', {
      params: { path },
    }),
    client.GET(
      '/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/revisions',
      {
        params: {
          path,
          query: {
            after: event.url.searchParams.get('after') ?? undefined,
            limit: 50,
          },
        },
      },
    ),
  ]);
  if (!entry.data || !revisions.data)
    error(
      revisions.response.status,
      revisions.error?.message ?? 'Revisions unavailable',
    );
  return {
    workspace,
    entry: entry.data,
    page: revisions.data,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  restore: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/revisions/{revision_id}/restore',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            entry_id: event.params.entry,
            revision_id: String(form.get('revision_id') ?? ''),
          },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: { expected_version: Number(form.get('expected_version')) },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not restore this revision',
      });
    if (result.data.dropped_fields.length)
      return {
        message: `Restored into the draft, without values for fields that no longer exist: ${result.data.dropped_fields.join(', ')}. They stay readable in the revision.`,
      };
    redirect(
      303,
      `/workspaces/${workspace.id}/cms/entries/${event.params.entry}`,
    );
  },
};

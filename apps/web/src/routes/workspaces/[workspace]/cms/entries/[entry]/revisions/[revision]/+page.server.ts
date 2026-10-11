import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/revisions/{revision_id}',
    {
      params: {
        path: {
          workspace_id: workspace.id,
          entry_id: event.params.entry,
          revision_id: event.params.revision,
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Revision unavailable',
    );
  return { workspace, revision: result.data };
};

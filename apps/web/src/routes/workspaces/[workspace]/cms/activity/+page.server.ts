import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/cms/contribution-events',
    { params: { path: { workspace_id: workspace.id } } },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Updates unavailable',
    );
  return { events: result.data };
};

import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const finished = event.url.searchParams.get('finished') === 'true';
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/my-work',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          include_finished: finished,
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Work unavailable');
  return { workspace, work: result.data, finished };
};

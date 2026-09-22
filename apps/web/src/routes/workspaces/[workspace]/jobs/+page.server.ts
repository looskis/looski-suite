import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const r = await client.GET('/api/v1/workspaces/{workspace_id}/jobs', {
    params: {
      path: { workspace_id: workspace.id },
      query: {
        limit: 25,
        after: event.url.searchParams.get('after') ?? undefined,
      },
    },
  });
  if (!r.data) error(r.response.status, r.error?.message ?? 'Jobs unavailable');
  return { workspace, page: r.data };
};

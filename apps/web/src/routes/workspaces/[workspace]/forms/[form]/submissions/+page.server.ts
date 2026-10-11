import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, form_id: event.params.form };
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}/submissions',
    {
      params: {
        path,
        query: {
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Submissions unavailable',
    );
  // Reading answers does not imply reading definitions; keys stay usable.
  const versions = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}/versions',
    { params: { path } },
  );
  return { workspace, page: result.data, versions: versions.data ?? [] };
};

import { json } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { RequestHandler } from './$types';
// Titles for the search box, under the signed-in member's own authority.
export const GET: RequestHandler = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const q = event.url.searchParams.get('q')?.trim() ?? '';
  if (!q) return json({ items: [] });
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/suggest',
    { params: { path: { workspace_id: workspace.id }, query: { q } } },
  );
  return json({ items: result.data?.items ?? [] });
};

import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import { exportSelection } from '$lib/server/export-selection';
import type { RequestHandler } from './$types';
export const GET: RequestHandler = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const catalog = await client.GET(
    '/api/v1/workspaces/{workspace_id}/exports/collections',
    { params: { path: { workspace_id: workspace.id } } },
  );
  if (!catalog.data)
    error(
      catalog.response.status,
      catalog.error?.message ?? 'Exports are unavailable',
    );
  const selection = exportSelection(event.url, catalog.data);
  const result = await client.POST(
    '/api/v1/workspaces/{workspace_id}/exports/page',
    { params: { path: { workspace_id: workspace.id } }, body: selection },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Could not export this collection',
    );
  return new Response(JSON.stringify(result.data, null, 2), {
    headers: {
      'Content-Type': 'application/json; charset=utf-8',
      'Content-Disposition': 'attachment; filename="looski-export-page.json"',
      'Cache-Control': 'no-store',
      'X-Content-Type-Options': 'nosniff',
    },
  });
};

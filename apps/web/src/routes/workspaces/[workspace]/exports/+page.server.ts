import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import { exportSelection } from '$lib/server/export-selection';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
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
  if (event.url.searchParams.get('run') !== 'true')
    return {
      workspace,
      collections: catalog.data,
      page: null,
      download: null,
      next: null,
    };
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
  const query = new URLSearchParams(event.url.searchParams);
  query.delete('run');
  const download = `/workspaces/${workspace.id}/exports/download?${query}`;
  let next: string | null = null;
  if (result.data.next?.after) {
    query.set('after', result.data.next.after);
    query.set('run', 'true');
    next = `?${query}`;
  }
  return {
    workspace,
    collections: catalog.data,
    page: result.data,
    download,
    next,
  };
};

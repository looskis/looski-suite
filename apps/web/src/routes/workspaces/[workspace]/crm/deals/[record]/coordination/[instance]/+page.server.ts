import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/coordination/{instance_id}',
    {
      params: {
        path: {
          workspace_id: workspace.id,
          id: event.params.record,
          instance_id: event.params.instance,
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Checklist unavailable',
    );
  return { workspace, instance: result.data };
};

import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
const statuses = ['open', 'in_progress', 'completed', 'cancelled'] as const;
const dues = ['overdue', 'soon', 'unscheduled'] as const;
const pick = <T extends string>(allowed: readonly T[], value: string | null) =>
  allowed.find((a) => a === value);
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const search = event.url.searchParams;
  const filters = {
    status: pick(statuses, search.get('status')),
    due: pick(dues, search.get('due')),
    assigned_membership_id: search.get('assignee') || undefined,
  };
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/{project_id}/delivery',
    {
      params: {
        path: { workspace_id: workspace.id, project_id: event.params.project },
        query: filters,
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Delivery view unavailable',
    );
  return { workspace, delivery: result.data, filters };
};

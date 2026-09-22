import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import { anchorKind } from '$lib/server/activity-form';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const kind = anchorKind(event.url.searchParams.get('anchor_kind')),
    id = event.url.searchParams.get('anchor_id');
  if (!kind || !id) error(400, 'Choose a contact, opportunity or deal');
  const r = await client.GET('/api/v1/workspaces/{workspace_id}/crm/timeline', {
    params: {
      path: { workspace_id: workspace.id },
      query: {
        anchor_kind: kind,
        anchor_id: id,
        limit: 25,
        after: event.url.searchParams.get('after') ?? undefined,
      },
    },
  });
  if (!r.data)
    error(r.response.status, r.error?.message ?? 'Timeline unavailable');
  return { workspace, page: r.data };
};

import { error } from '@sveltejs/kit';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}/summary',
    {
      params: {
        path: { workspace_id: workspace.id, form_id: event.params.form },
      },
    },
  );
  // A form that was never published has nothing to summarize yet.
  if (!result.data && result.response.status !== 409)
    error(
      result.response.status,
      result.error?.message ?? 'Summary unavailable',
    );
  return {
    workspace,
    summary: result.data ?? null,
    message: result.error?.message ?? null,
  };
};

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
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}/submissions/{submission_id}',
    {
      params: {
        path: { ...path, submission_id: event.params.submission },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Submission unavailable',
    );
  const versions = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}/versions',
    { params: { path } },
  );
  return {
    workspace,
    submission: result.data,
    version:
      versions.data?.find((v) => v.id === result.data.version_id) ?? null,
  };
};

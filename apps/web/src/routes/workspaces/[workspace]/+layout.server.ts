import { redirect } from '@sveltejs/kit';
import { workspaceClient, signedInEmail } from '$lib/server/session';
import { readAppearance } from '$lib/server/appearance';
import type { LayoutServerLoad } from './$types';
export const load: LayoutServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const writing = await client.GET(
    '/api/v1/workspaces/{workspace_id}/cms/writing-access',
    { params: { path: { workspace_id: workspace.id } } },
  );
  if (
    writing.data?.contributor_only &&
    event.url.pathname === `/workspaces/${workspace.id}`
  )
    redirect(303, `/workspaces/${workspace.id}/cms`);
  return {
    writing: writing.data ?? null,
    workspace,
    accountEmail: await signedInEmail(event),
    appearance: readAppearance(event.cookies),
  };
};

import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const r = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/starter-packages/{id}',
    {
      params: {
        path: { workspace_id: workspace.id, id: event.params.installation },
      },
    },
  );
  if (!r.data)
    error(r.response.status, r.error?.message ?? 'Installation unavailable');
  return { workspace, installation: r.data, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData(),
      draft = {
        key: String(f.get('idempotency_key') ?? ''),
        expected_version: String(f.get('expected_version') ?? ''),
      };
    const version = Number(draft.expected_version);
    if (!Number.isSafeInteger(version) || version < 1)
      return fail(400, { message: 'Valid version required', draft });
    const r = await client.POST(
      '/api/v1/workspaces/{workspace_id}/crm/starter-packages/{id}/remove',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.installation },
          header: { 'Idempotency-Key': draft.key },
        },
        body: { expected_version: version },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not remove package',
        draft,
      });
    redirect(303, event.url.pathname);
  },
};

import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { roleInput } from '$lib/server/access-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const [role, permissions] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/access/roles/{role_id}', {
      params: {
        path: { workspace_id: workspace.id, role_id: event.params.role },
      },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/access/permissions', {
      params: { path: { workspace_id: workspace.id } },
    }),
  ]);
  if (!role.data)
    error(role.response.status, role.error?.message ?? 'Role unavailable');
  if (!permissions.data)
    error(
      permissions.response.status,
      permissions.error?.message ?? 'Permissions unavailable',
    );
  return {
    workspace,
    role: role.data,
    permissions: permissions.data,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/access/roles/{role_id}',
      {
        params: {
          path: { workspace_id: workspace.id, role_id: event.params.role },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: roleInput(form),
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not update role',
      });
    redirect(303, event.url.pathname);
  },
};

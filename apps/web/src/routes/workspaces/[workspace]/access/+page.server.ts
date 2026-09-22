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
  const [roles, members, permissions] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/access/roles', {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          after: event.url.searchParams.get('roles_after') ?? undefined,
          limit: 50,
        },
      },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/access/members', {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          after: event.url.searchParams.get('members_after') ?? undefined,
          limit: 50,
        },
      },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/access/permissions', {
      params: { path: { workspace_id: workspace.id } },
    }),
  ]);
  if (!roles.data)
    error(roles.response.status, roles.error?.message ?? 'Roles unavailable');
  if (!members.data)
    error(
      members.response.status,
      members.error?.message ?? 'Members unavailable',
    );
  if (!permissions.data)
    error(
      permissions.response.status,
      permissions.error?.message ?? 'Permissions unavailable',
    );
  return {
    workspace,
    roles: roles.data,
    members: members.data,
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
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/access/roles',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: roleInput(form),
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create role',
      });
    redirect(303, `/workspaces/${workspace.id}/access/roles/${result.data.id}`);
  },
};

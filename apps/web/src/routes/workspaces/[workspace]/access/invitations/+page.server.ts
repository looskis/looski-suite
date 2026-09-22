import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const selected = [...new Set(event.url.searchParams.getAll('role_ids'))];
  if (
    selected.length > 50 ||
    selected.some((id) => !/^[-0-9a-f]{36}$/.test(id))
  )
    error(400, 'Choose at most 50 valid roles');
  const [invitations, roles, ...chosen] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/access/invitations', {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          limit: 50,
          after: event.url.searchParams.get('after') ?? undefined,
        },
      },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/access/roles', {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          limit: 50,
          after: event.url.searchParams.get('roles_after') ?? undefined,
        },
      },
    }),
    ...selected.map((role_id) =>
      client.GET('/api/v1/workspaces/{workspace_id}/access/roles/{role_id}', {
        params: { path: { workspace_id: workspace.id, role_id } },
      }),
    ),
  ]);
  if (!invitations.data)
    error(
      invitations.response.status,
      invitations.error?.message ?? 'Invitations unavailable',
    );
  if (!roles.data)
    error(roles.response.status, roles.error?.message ?? 'Roles unavailable');
  const chosenRoles = chosen.map((r) => {
    if (!r.data)
      error(r.response.status, r.error?.message ?? 'Selected role unavailable');
    return r.data;
  });
  return {
    workspace,
    invitations: invitations.data,
    roles: roles.data,
    selected: chosenRoles,
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
    const draft = {
      email: String(form.get('email') ?? ''),
      expires_in_hours: String(form.get('expires_in_hours') ?? ''),
      role_ids: form.getAll('role_ids').map(String),
    };
    const idempotency_key = String(form.get('idempotency_key') ?? '');
    const hours = Number(draft.expires_in_hours);
    if (!Number.isInteger(hours) || hours < 1 || hours > 168)
      return fail(400, {
        message: 'Expiry must be an integer from 1 to 168 hours',
        draft,
        key: idempotency_key,
      });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/access/invitations',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': idempotency_key },
        },
        body: { ...draft, expires_in_hours: hours },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create invitation',
        draft,
        key: idempotency_key,
      });
    redirect(
      303,
      `/workspaces/${workspace.id}/access/invitations/${result.data.invitation.id}`,
    );
  },
};

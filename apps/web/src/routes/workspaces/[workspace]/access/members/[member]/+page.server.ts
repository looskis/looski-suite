import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const [member, roles] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/access/members/{member_id}', {
      params: {
        path: { workspace_id: workspace.id, member_id: event.params.member },
      },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/access/roles', {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          limit: 100,
          after: event.url.searchParams.get('after') ?? undefined,
        },
      },
    }),
  ]);
  if (!member.data)
    error(
      member.response.status,
      member.error?.message ?? 'Member unavailable',
    );
  if (!roles.data)
    error(roles.response.status, roles.error?.message ?? 'Roles unavailable');
  // Preserve assigned roles outside this catalog page; paging must never silently revoke grants.
  const choices = new Map(
    roles.data.items.map((r) => [
      r.id,
      { id: r.id, name: r.name, archived: r.archived },
    ]),
  );
  for (const r of member.data.roles) choices.set(r.id, r);
  return {
    workspace,
    member: member.data,
    roles: [...choices.values()],
    next: roles.data.next_cursor,
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
    const state = String(form.get('state'));
    if (state !== 'active' && state !== 'suspended' && state !== 'removed')
      return fail(400, { message: 'Choose a supported state' });
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/access/members/{member_id}',
      {
        params: {
          path: { workspace_id: workspace.id, member_id: event.params.member },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          state,
          role_ids:
            state === 'removed' ? [] : form.getAll('role_ids').map(String),
          expected_version: Number(form.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not update member',
      });
    redirect(303, event.url.pathname);
  },
};

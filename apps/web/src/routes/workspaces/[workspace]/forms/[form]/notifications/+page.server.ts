import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}',
    {
      params: {
        path: { workspace_id: workspace.id, form_id: event.params.form },
      },
    },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Form unavailable');
  const members = await client.GET(
    '/api/v1/workspaces/{workspace_id}/access/members',
    { params: { path: { workspace_id: workspace.id }, query: { limit: 100 } } },
  );
  if (!members.data && members.response.status !== 403)
    error(
      members.response.status,
      members.error?.message ?? 'Members unavailable',
    );
  const active =
    members.data?.items
      .filter((m) => m.state === 'active')
      .map((m) => ({ id: m.id, name: m.display_name })) ?? [];
  // A current recipient stays selectable when listing members is not permitted.
  for (const id of result.data.notification_membership_ids)
    if (!active.some((m) => m.id === id)) active.push({ id, name: id });
  return {
    workspace,
    form: result.data,
    members: active,
    membersHidden: !members.data,
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
    const membership_ids = form.getAll('membership_ids').map(String);
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.PUT(
      '/api/v1/workspaces/{workspace_id}/forms/{form_id}/notifications',
      {
        params: {
          path: { workspace_id: workspace.id, form_id: event.params.form },
          header: { 'Idempotency-Key': key },
        },
        body: {
          expected_version: Number(form.get('expected_version')),
          membership_ids,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save recipients',
        membership_ids,
        key,
      });
    redirect(303, event.url.pathname);
  },
};

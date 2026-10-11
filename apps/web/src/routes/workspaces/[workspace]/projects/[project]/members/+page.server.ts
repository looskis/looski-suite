import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, project_id: event.params.project };
  const members = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/{project_id}/members',
    { params: { path } },
  );
  if (!members.data)
    error(
      members.response.status,
      members.error?.message ?? 'Members unavailable',
    );
  // Only members who can add others see the directory.
  const directory = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/directory',
    { params: { path: { workspace_id: workspace.id } } },
  );
  const current = new Set(members.data.map((m) => m.id));
  return {
    workspace,
    members: members.data,
    canManage: Boolean(directory.data),
    candidates: (directory.data ?? []).filter((m) => !current.has(m.id)),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  add: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/members',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            project_id: event.params.project,
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: { membership_id: String(form.get('membership_id') ?? '') },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not add the member',
      });
    redirect(303, event.url.pathname);
  },
  remove: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/{project_id}/members/{membership_id}/remove',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            project_id: event.params.project,
            membership_id: String(form.get('membership_id') ?? ''),
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not remove the member',
      });
    redirect(303, event.url.pathname);
  },
};

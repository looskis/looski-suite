import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id };
  const [grants, types, members, media] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/cms/contributors', {
      params: { path },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/cms/types', {
      params: { path, query: { limit: 100 } },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/access/members', {
      params: { path, query: { limit: 100 } },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/cms/media', {
      params: { path, query: { limit: 100 } },
    }),
  ]);
  if (!grants.data)
    error(
      grants.response.status,
      grants.error?.message ?? 'Profiles unavailable',
    );
  return {
    grants: grants.data,
    types: types.data?.items ?? [],
    members: members.data?.items ?? [],
    media:
      media.data?.items.filter((m) => m.media_type.startsWith('image/')) ?? [],
    key: randomUUID(),
  };
};
export const actions: Actions = {
  role: async (event) => {
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
        body: {
          name: 'Contributor',
          grants: [
            { action: 'workspace.read', scope: 'workspace' },
            { action: 'cms.contribute', scope: 'workspace' },
            { action: 'files.read', scope: 'assigned' },
            { action: 'files.create', scope: 'assigned' },
          ],
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create contributor role',
      });
    redirect(303, `/workspaces/${workspace.id}/access/invitations`);
  },
  save: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/cms/contributors',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          membership_id: String(form.get('membership_id')),
          type_id: String(form.get('type_id')),
          name: String(form.get('name') ?? ''),
          bio: String(form.get('bio') ?? ''),
          photo_media_id: String(form.get('photo_media_id') ?? '') || null,
          active: form.get('active') === 'true',
          expected_version: Number(form.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save profile',
        input: Object.fromEntries(form),
      });
    redirect(303, event.url.pathname);
  },
};

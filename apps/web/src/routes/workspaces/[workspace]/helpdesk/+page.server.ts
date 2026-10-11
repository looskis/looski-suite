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
  const path = { workspace_id: workspace.id };
  const modules = await client.GET(
    '/api/v1/workspaces/{workspace_id}/modules',
    { params: { path } },
  );
  const module = modules.data?.find((m) => m.key === 'helpdesk') ?? null;
  const archived = event.url.searchParams.get('archived') === 'true';
  const status = event.url.searchParams.get('status') || undefined;
  const base = {
    workspace,
    module,
    archived,
    status: status ?? '',
    key: randomUUID(),
    origin: event.url.origin,
  };
  if (module && !module.enabled)
    return { ...base, tickets: null, settings: null };
  const tickets = await client.GET(
    '/api/v1/workspaces/{workspace_id}/helpdesk/tickets',
    {
      params: {
        path,
        query: {
          archived,
          status,
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!tickets.data)
    error(
      tickets.response.status,
      tickets.error?.message ?? 'Helpdesk unavailable',
    );
  const settings = await client.GET(
    '/api/v1/workspaces/{workspace_id}/helpdesk/settings',
    { params: { path } },
  );
  return { ...base, tickets: tickets.data, settings: settings.data ?? null };
};
export const actions: Actions = {
  module: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/modules/{module_key}',
      {
        params: {
          path: { workspace_id: workspace.id, module_key: 'helpdesk' },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          enabled: f.get('enabled') === 'true',
          expected_version: Number(f.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change selection',
      });
    redirect(303, event.url.pathname);
  },
  configure: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const result = await client.PUT(
      '/api/v1/workspaces/{workspace_id}/helpdesk/settings',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          accepting: f.get('accepting') === 'true',
          expected_version: Number(f.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save settings',
      });
    redirect(303, event.url.pathname);
  },
};

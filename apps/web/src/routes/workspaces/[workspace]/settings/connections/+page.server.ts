import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey, text } from '$lib/server/project-form';
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
  const module = modules.data?.find((m) => m.key === 'communications') ?? null;
  const archived = event.url.searchParams.get('archived') === 'true';
  const contact_id = event.url.searchParams.get('contact_id') || undefined;
  const base = { workspace, module, archived, contact_id, key: randomUUID() };
  if (module && !module.enabled)
    return { ...base, conversations: null, accounts: [] };
  const accounts = await client.GET(
    '/api/v1/workspaces/{workspace_id}/communications/accounts',
    { params: { path } },
  );
  return {
    ...base,
    conversations: true,
    accounts: accounts.data ?? [],
  };
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
          path: { workspace_id: workspace.id, module_key: 'communications' },
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
        message: result.error?.message ?? 'Could not change application',
      });
    redirect(303, event.url.pathname);
  },
  connect: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const input = {
      name: text(f, 'name'),
      connector_key: text(f, 'connector_key'),
      shared: f.get('shared') === 'on',
    };
    const key = retryKey(f);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/communications/accounts',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not connect',
        input,
        key,
      });
    redirect(303, event.url.pathname);
  },
  account: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/communications/accounts/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: text(f, 'id') },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          name: text(f, 'name'),
          enabled: f.get('enabled') === 'on',
          shared: f.get('shared') === 'on',
          expected_version: Number(f.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not update connection',
      });
    redirect(303, event.url.pathname);
  },
  sync: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/communications/accounts/{id}/sync',
      { params: { path: { workspace_id: workspace.id, id: text(f, 'id') } } },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not refresh connection',
      });
    redirect(303, event.url.pathname);
  },
};

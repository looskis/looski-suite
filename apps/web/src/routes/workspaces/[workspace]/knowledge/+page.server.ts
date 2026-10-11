import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey, text, version } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id };
  // Selection is an administrator's read; other members simply see the library.
  const modules = await client.GET(
    '/api/v1/workspaces/{workspace_id}/modules',
    { params: { path } },
  );
  const module = modules.data?.find((m) => m.key === 'knowledge') ?? null;
  const params = event.url.searchParams;
  const q = params.get('q')?.trim() ?? '';
  const archived = params.get('archived') === 'true';
  const base = { workspace, module, q, archived, key: randomUUID() };
  if (module && !module.enabled)
    return { ...base, collections: null, results: null, activity: null };
  const collections = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/collections',
    {
      params: {
        path,
        query: {
          archived,
          after: params.get('collections_after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!collections.data)
    error(
      collections.response.status,
      collections.error?.message ?? 'Knowledge base unavailable',
    );
  const results = q
    ? await client.GET('/api/v1/workspaces/{workspace_id}/knowledge/search', {
        params: {
          path,
          query: { q, after: params.get('after') ?? undefined, limit: 20 },
        },
      })
    : null;
  const activity = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/activity',
    { params: { path, query: { limit: 10 } } },
  );
  return {
    ...base,
    collections: collections.data,
    results: results?.data ?? null,
    searchError: results?.error?.message ?? null,
    activity: activity.data?.items ?? [],
  };
};
export const actions: Actions = {
  createCollection: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const input = {
      name: text(form, 'name'),
      description: text(form, 'description'),
      restricted: form.get('restricted') === 'on',
    };
    const key = retryKey(form);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/knowledge/collections',
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
        message: result.error?.message ?? 'Could not create the collection',
        input,
        key,
      });
    redirect(
      303,
      `/workspaces/${workspace.id}/knowledge/collections/${result.data.id}`,
    );
  },
  module: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/modules/{module_key}',
      {
        params: {
          path: { workspace_id: workspace.id, module_key: 'knowledge' },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: {
          enabled: form.get('enabled') === 'true',
          expected_version: Number(form.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message:
          result.error?.message ?? 'Could not change application selection',
      });
    redirect(303, event.url.pathname);
  },
};

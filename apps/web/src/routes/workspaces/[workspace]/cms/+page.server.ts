import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { slugFrom } from '$lib/server/cms-form';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
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
  const module = modules.data?.find((m) => m.key === 'cms') ?? null;
  const search = event.url.searchParams.get('search') ?? '';
  const archived = event.url.searchParams.get('archived') === 'true';
  const typeId = event.url.searchParams.get('type') ?? '';
  const base = { workspace, module, search, archived, typeId };
  if (module && !module.enabled)
    return { ...base, page: null, types: [], key: randomUUID() };
  const [entries, types] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/cms/entries', {
      params: {
        path,
        query: {
          search: search || undefined,
          archived,
          type_id: typeId || undefined,
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/cms/types', {
      params: { path, query: { limit: 100 } },
    }),
  ]);
  if (!entries.data)
    error(
      entries.response.status,
      entries.error?.message ?? 'Content unavailable',
    );
  return {
    ...base,
    page: entries.data,
    types: types.data?.items ?? [],
    key: randomUUID(),
  };
};
export const actions: Actions = {
  create: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const title = String(form.get('title') ?? '');
    const slug = String(form.get('slug') ?? '').trim() || slugFrom(title);
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/cms/entries',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': key },
        },
        body: { type_id: String(form.get('type_id') ?? ''), title, slug },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create this entry',
        input: { title, slug },
        key,
      });
    redirect(303, `/workspaces/${workspace.id}/cms/entries/${result.data.id}`);
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
          path: { workspace_id: workspace.id, module_key: 'cms' },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
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

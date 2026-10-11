import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { dateOrNull, retryKey, text } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id };
  // Selection is an administrator's read; other members simply see the list.
  const modules = await client.GET(
    '/api/v1/workspaces/{workspace_id}/modules',
    { params: { path } },
  );
  const module = modules.data?.find((m) => m.key === 'projects') ?? null;
  const search = event.url.searchParams.get('search') ?? '';
  const archived = event.url.searchParams.get('archived') === 'true';
  const base = { workspace, module, search, archived, key: randomUUID() };
  if (module && !module.enabled) return { ...base, page: null };
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects',
    {
      params: {
        path,
        query: {
          search: search || undefined,
          archived,
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Projects unavailable',
    );
  return { ...base, page: result.data };
};
export const actions: Actions = {
  create: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const input = {
      name: text(form, 'name'),
      description: text(form, 'description'),
      due_on: dateOrNull(form, 'due_on'),
    };
    const key = retryKey(form);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects',
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
        message: result.error?.message ?? 'Could not create project',
        input,
        key,
      });
    redirect(303, `/workspaces/${workspace.id}/projects/${result.data.id}`);
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
          path: { workspace_id: workspace.id, module_key: 'projects' },
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

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
  // Selection is an administrator's read; other members simply see the list.
  const modules = await client.GET(
    '/api/v1/workspaces/{workspace_id}/modules',
    { params: { path } },
  );
  const module = modules.data?.find((m) => m.key === 'forms') ?? null;
  const search = event.url.searchParams.get('search') ?? '';
  const archived = event.url.searchParams.get('archived') === 'true';
  if (module && !module.enabled)
    return {
      workspace,
      module,
      page: null,
      search,
      archived,
      key: randomUUID(),
    };
  const result = await client.GET('/api/v1/workspaces/{workspace_id}/forms', {
    params: {
      path,
      query: {
        search: search || undefined,
        archived,
        after: event.url.searchParams.get('after') ?? undefined,
        limit: 50,
      },
    },
  });
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Forms unavailable');
  return {
    workspace,
    module,
    page: result.data,
    search,
    archived,
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
    const input = {
      name: String(form.get('name') ?? ''),
      description: String(form.get('description') ?? '').replaceAll(
        '\r\n',
        '\n',
      ),
      confirmation_message: String(
        form.get('confirmation_message') ?? '',
      ).replaceAll('\r\n', '\n'),
    };
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/forms',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': key },
        },
        body: { ...input, draft: { fields: [] } },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create form',
        input,
        key,
      });
    redirect(303, `/workspaces/${workspace.id}/forms/${result.data.id}`);
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
          path: { workspace_id: workspace.id, module_key: 'forms' },
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

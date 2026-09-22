import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  configurationPaths,
  definitionInput,
} from '$lib/server/crm-configuration';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const paths = configurationPaths(event.params.kind);
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const archived = event.url.searchParams.get('archived') === 'true';
  const result = await client.GET(paths.list, {
    params: {
      path: { workspace_id: workspace.id },
      query: {
        limit: 50,
        after: event.url.searchParams.get('after') ?? undefined,
        archived,
      },
    },
  });
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Configuration unavailable',
    );
  const parents = paths.parent
    ? await client.GET(paths.parent, {
        params: {
          path: { workspace_id: workspace.id },
          query: {
            limit: 100,
            after: event.url.searchParams.get('parents_after') ?? undefined,
          },
        },
      })
    : null;
  if (parents && !parents.data)
    error(
      parents.response.status,
      parents.error?.message ?? 'Parent definitions unavailable',
    );
  return {
    workspace,
    kind: event.params.kind,
    title: paths.title,
    page: result.data,
    parents: parents?.data?.items ?? [],
    parents_next: parents?.data?.next_cursor,
    archived,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const paths = configurationPaths(event.params.kind);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(paths.list, {
      params: {
        path: { workspace_id: workspace.id },
        header: {
          'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
        },
      },
      body: definitionInput(form),
    });
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create definition',
        draft: definitionInput(form),
        key: String(form.get('idempotency_key') ?? ''),
      });
    redirect(303, event.url.pathname + '/' + result.data.id);
  },
};

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
  const result = await client.GET(paths.item, {
    params: {
      path: { workspace_id: workspace.id, id: event.params.definition },
    },
  });
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Definition unavailable',
    );
  return {
    workspace,
    kind: event.params.kind,
    definition: result.data,
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
    const result = await client.PATCH(paths.item, {
      params: {
        path: { workspace_id: workspace.id, id: event.params.definition },
        header: {
          'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
        },
      },
      body: definitionInput(form),
    });
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not update definition',
        draft: definitionInput(form),
        key: String(form.get('idempotency_key') ?? ''),
      });
    redirect(303, event.url.pathname);
  },
};

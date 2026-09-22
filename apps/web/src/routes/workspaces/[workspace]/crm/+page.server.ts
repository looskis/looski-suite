import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET('/api/v1/workspaces/{workspace_id}/modules', {
    params: { path: { workspace_id: workspace.id } },
  });
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Applications unavailable',
    );
  const module = result.data.find((m) => m.key === 'crm');
  if (!module) error(404, 'CRM unavailable');
  return { workspace, module, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/modules/crm',
      {
        params: {
          path: { workspace_id: workspace.id },
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

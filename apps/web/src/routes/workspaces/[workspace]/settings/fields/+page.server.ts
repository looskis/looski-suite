import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { recordFields } from '$lib/server/record-views';
import type { PageServerLoad, Actions } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const kind = event.url.searchParams.get('kind') ?? 'contacts';
  if (!['contacts', 'organizations'].includes(kind))
    error(400, 'Unknown record type');
  return {
    workspace,
    kind,
    fields: await recordFields(client, workspace.id, kind),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  save: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const kind = String(form.get('kind'));
    const input = {
      key: String(form.get('key')),
      name: String(form.get('name')),
      field_type: String(form.get('field_type')),
      options: String(form.get('options') ?? '')
        .split('\n')
        .map((s) => s.trim())
        .filter(Boolean),
      archived: form.get('archived') === 'on',
      expected_version: form.get('version')
        ? Number(form.get('version'))
        : null,
    };
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/records/{kind}/fields',
      {
        params: {
          path: { workspace_id: workspace.id, kind },
          header: { 'Idempotency-Key': String(form.get('idempotency_key')) },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save field',
        input,
      });
    redirect(303, `${event.url.pathname}?kind=${kind}`);
  },
};

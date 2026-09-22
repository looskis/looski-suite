import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}',
    {
      params: {
        path: { workspace_id: workspace.id, contact_id: event.params.contact },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Contact unavailable',
    );
  if (result.data.id !== event.params.contact)
    error(409, 'Merged contacts cannot be permanently deleted');
  return { workspace, contact: result.data, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const draft = {
      expected_version: String(form.get('expected_version') ?? ''),
      confirm_id: String(form.get('confirm_id') ?? ''),
    };
    const key = String(form.get('idempotency_key') ?? '');
    const version = Number(draft.expected_version);
    if (
      !Number.isSafeInteger(version) ||
      version < 1 ||
      draft.confirm_id !== event.params.contact
    )
      return fail(400, {
        message: 'Enter this contact’s exact ID to confirm permanent deletion.',
        draft,
        key,
      });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/delete-permanently',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            contact_id: event.params.contact,
          },
          header: { 'Idempotency-Key': key },
        },
        body: { expected_version: version, confirm_id: draft.confirm_id },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not delete contact',
        draft,
        key,
      });
    redirect(303, `/workspaces/${workspace.id}/contacts`);
  },
};

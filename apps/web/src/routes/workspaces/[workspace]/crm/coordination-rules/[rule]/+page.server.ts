import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { ruleFields, ruleDraft, ruleInput } from '$lib/server/rule-form';
import type { PageServerLoad, Actions } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const r = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/coordination-rules/{id}',
    { params: { path: { workspace_id: workspace.id, id: event.params.rule } } },
  );
  if (!r.data) error(r.response.status, r.error?.message ?? 'Rule unavailable');
  return {
    workspace,
    rule: r.data,
    fields: ruleFields(r.data),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData(),
      draft = ruleDraft(f),
      input = ruleInput(f, true);
    if (!input)
      return fail(400, {
        message: 'Valid availability and version required',
        draft,
      });
    const r = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/crm/coordination-rules/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.rule },
          header: { 'Idempotency-Key': draft.key },
        },
        body: input,
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not save rule',
        draft,
      });
    redirect(303, event.url.pathname);
  },
};

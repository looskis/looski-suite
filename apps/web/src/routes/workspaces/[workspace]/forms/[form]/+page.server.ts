import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { applyOperation, builderDraft } from '$lib/server/form-builder';
import type { Actions, PageServerLoad, RequestEvent } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}',
    {
      params: {
        path: { workspace_id: workspace.id, form_id: event.params.form },
      },
    },
  );
  if (!result.data)
    error(result.response.status, result.error?.message ?? 'Form unavailable');
  return {
    workspace,
    form: result.data,
    key: randomUUID(),
    origin: event.url.origin,
    notice: event.url.searchParams.get('notice'),
  };
};
async function setup(event: RequestEvent) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const form = await event.request.formData();
  return {
    client,
    form,
    key: String(form.get('idempotency_key') ?? ''),
    params: { workspace_id: workspace.id, form_id: event.params.form },
    expected_version: Number(form.get('expected_version')),
  };
}
export const actions: Actions = {
  builder: async (event) => {
    const { client, form, key, params } = await setup(event);
    const draft = builderDraft(form);
    const operation = String(form.get('op') ?? 'save');
    const applied = applyOperation(form, draft, operation);
    if ('message' in applied)
      return fail(400, {
        message: applied.message,
        builder: draft,
        key,
        stale: false,
      });
    const saved = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/forms/{form_id}',
      {
        params: { path: params, header: { 'Idempotency-Key': key } },
        body: {
          name: draft.name,
          description: draft.description,
          confirmation_message: draft.confirmation_message,
          draft: { fields: applied.fields },
          expected_version: Number(draft.expected_version),
        },
      },
    );
    if (!saved.data)
      return fail(saved.response.status, {
        message: saved.error?.message ?? 'Could not save this form',
        builder: draft,
        key,
        stale: saved.response.status === 409,
      });
    if (operation === 'publish') {
      const published = await client.POST(
        '/api/v1/workspaces/{workspace_id}/forms/{form_id}/publish',
        {
          params: {
            path: params,
            header: { 'Idempotency-Key': `${key}-publish` },
          },
          body: { expected_version: saved.data.version },
        },
      );
      if (!published.data)
        return fail(published.response.status, {
          message: `Your draft was saved but not published: ${published.error?.message ?? 'please retry'}`,
          builder: {
            ...draft,
            fields: applied.fields,
            expected_version: String(saved.data.version),
            addition: { ...draft.addition, label: '', key: '', options: '' },
          },
          key: randomUUID(),
          stale: false,
        });
      redirect(303, `${event.url.pathname}?notice=published`);
    }
    redirect(303, `${event.url.pathname}?notice=saved`);
  },
  accepting: async (event) => {
    const { client, form, key, params, expected_version } = await setup(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/forms/{form_id}/accepting',
      {
        params: { path: params, header: { 'Idempotency-Key': key } },
        body: { expected_version, accepting: form.get('accepting') === 'true' },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change this form',
        key,
        stale: result.response.status === 409,
      });
    redirect(303, event.url.pathname);
  },
  archive: async (event) => {
    const { client, form, key, params, expected_version } = await setup(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/forms/{form_id}/archive',
      {
        params: { path: params, header: { 'Idempotency-Key': key } },
        body: { expected_version, archived: form.get('archived') === 'true' },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change this form',
        key,
        stale: result.response.status === 409,
      });
    redirect(303, event.url.pathname);
  },
};

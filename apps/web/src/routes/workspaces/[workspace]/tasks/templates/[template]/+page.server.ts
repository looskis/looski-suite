import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  templateFields,
  templateDraft,
  templateInput,
} from '$lib/server/task-template-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, id: event.params.template };
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/tasks/templates/{id}',
    { params: { path } },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Template unavailable',
    );
  const revisions = await client.GET(
    '/api/v1/workspaces/{workspace_id}/tasks/templates/{id}/revisions',
    {
      params: {
        path,
        query: {
          limit: 50,
          after: event.url.searchParams.get('after') ?? undefined,
        },
      },
    },
  );
  if (!revisions.data)
    error(
      revisions.response.status,
      revisions.error?.message ?? 'Revisions unavailable',
    );
  return {
    workspace,
    template: result.data,
    revisions: revisions.data,
    fields: templateFields(result.data),
    key: randomUUID(),
    instantiateKey: randomUUID(),
    today: new Date().toISOString().slice(0, 10),
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
    const draft = templateDraft(form);
    const input = templateInput(form, true);
    if (!input)
      return fail(400, {
        message: 'Valid assignment, offset, availability and version required',
        draft,
      });
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/tasks/templates/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.template },
          header: { 'Idempotency-Key': draft.key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not update template',
        draft,
      });
    redirect(303, event.url.pathname);
  },
  instantiate: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const t = (k: string) => String(form.get(k) ?? '');
    const instantiateDraft = {
      revision_id: t('revision_id'),
      base_on: t('base_on'),
      context_owner_membership_id: t('context_owner_membership_id'),
      key: t('idempotency_key'),
    };
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/tasks/templates/{id}/instantiate',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.template },
          header: { 'Idempotency-Key': instantiateDraft.key },
        },
        body: {
          revision_id: instantiateDraft.revision_id,
          base_on: instantiateDraft.base_on,
          context_owner_membership_id:
            instantiateDraft.context_owner_membership_id || null,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create task',
        instantiateDraft,
      });
    redirect(303, `/workspaces/${workspace.id}/tasks/${result.data.id}`);
  },
};

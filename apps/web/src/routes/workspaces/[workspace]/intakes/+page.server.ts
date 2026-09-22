import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { contactForm } from '$lib/server/contact-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const state = event.url.searchParams.get('state') ?? 'pending';
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/contact-intakes',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          state,
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Intake unavailable',
    );
  return { workspace, page: result.data, state, key: randomUUID() };
};
export const actions: Actions = {
  submit: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const proposed = contactForm(form);
    proposed.allow_shared = false;
    const input = {
      source_ref: String(form.get('source_ref') ?? ''),
      proposed,
    };
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/contact-intakes',
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
        message: result.error?.message ?? 'Could not submit intake',
        input,
        key,
      });
    redirect(303, `/workspaces/${workspace.id}/intakes/${result.data.id}`);
  },
};

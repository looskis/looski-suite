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
  const search = event.url.searchParams.get('search') ?? '';
  const archived = event.url.searchParams.get('archived') === 'true';
  const after = event.url.searchParams.get('after') ?? undefined;
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/contacts',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: { search, archived, after, limit: 50 },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Contacts are unavailable',
    );
  return {
    workspace,
    page: result.data,
    search,
    archived,
    idempotencyKey: randomUUID(),
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
    const input = contactForm(form);
    const idempotencyKey = String(form.get('idempotency_key') ?? '');
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/contacts',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': idempotencyKey },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create contact',
        input,
        idempotencyKey,
      });
    redirect(303, `/workspaces/${workspace.id}/contacts/${result.data.id}`);
  },
};

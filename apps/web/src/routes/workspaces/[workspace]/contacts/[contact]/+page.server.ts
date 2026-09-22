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
  return {
    workspace,
    contact: result.data,
    editKey: randomUUID(),
    archiveKey: randomUUID(),
  };
};
export const actions: Actions = {
  edit: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const input = contactForm(form);
    const idempotencyKey = String(form.get('idempotency_key') ?? '');
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            contact_id: event.params.contact,
          },
          header: { 'Idempotency-Key': idempotencyKey },
        },
        body: {
          ...input,
          expected_version: Number(form.get('expected_version')),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save contact',
        input,
        idempotencyKey,
        expectedVersion: Number(form.get('expected_version')),
      });
    redirect(303, event.url.pathname);
  },
  archive: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/archive',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            contact_id: event.params.contact,
          },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          expected_version: Number(form.get('expected_version')),
          archived: form.get('archived') === 'true',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change archive state',
      });
    redirect(303, event.url.pathname);
  },
};

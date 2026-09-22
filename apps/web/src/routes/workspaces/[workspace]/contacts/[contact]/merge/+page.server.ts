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
  const get = async (id: string) => {
    const result = await client.GET(
      '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}',
      { params: { path: { workspace_id: workspace.id, contact_id: id } } },
    );
    if (!result.data)
      error(
        result.response.status,
        result.error?.message ?? 'Contact unavailable',
      );
    return result.data;
  };
  const survivor = await get(event.params.contact);
  if (survivor.id !== event.params.contact)
    redirect(
      303,
      `/workspaces/${workspace.id}/contacts/${survivor.id}/merge${event.url.search}`,
    );
  const sourceId = event.url.searchParams.get('source');
  const source = sourceId ? await get(sourceId) : null;
  const relationships = async (id: string) => {
    const result = await client.GET(
      '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/organizations',
      {
        params: { path: { workspace_id: workspace.id, contact_id: id } },
      },
    );
    if (!result.data && result.response.status !== 403)
      error(
        result.response.status,
        result.error?.message ?? 'Relationships unavailable',
      );
    return result.data ?? null;
  };
  const survivorRelationships = await relationships(survivor.id);
  const sourceRelationships = source ? await relationships(source.id) : null;
  return {
    workspace,
    survivor,
    source,
    survivorRelationships,
    sourceRelationships,
    idempotencyKey: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    if (form.get('confirmed') !== 'on')
      return fail(400, {
        message: 'Confirm these records represent the same person.',
      });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/merge',
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
          source_id: String(form.get('source_id')),
          source_version: Number(form.get('source_version')),
          survivor_version: Number(form.get('survivor_version')),
          resolved: contactForm(form),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message:
          result.error?.message ??
          'Merge failed. Reload the preview before retrying.',
      });
    redirect(303, `/workspaces/${workspace.id}/contacts/${result.data.id}`);
  },
};

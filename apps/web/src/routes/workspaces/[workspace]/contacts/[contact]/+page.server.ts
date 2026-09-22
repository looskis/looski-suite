import { tagPanel, saveTags } from '$lib/server/tag-panel';
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
  if (result.data.id !== event.params.contact)
    redirect(303, `/workspaces/${workspace.id}/contacts/${result.data.id}`);
  const organizationSearch =
    event.url.searchParams.get('organization_search') ?? '';
  const relationships = await client.GET(
    '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/organizations',
    {
      params: {
        path: { workspace_id: workspace.id, contact_id: result.data.id },
      },
    },
  );
  if (!relationships.data && relationships.response.status !== 403)
    error(
      relationships.response.status,
      relationships.error?.message ?? 'Relationships unavailable',
    );
  const organizations = relationships.data
    ? await client.GET('/api/v1/workspaces/{workspace_id}/organizations', {
        params: {
          path: { workspace_id: workspace.id },
          query: { search: organizationSearch, limit: 50 },
        },
      })
    : null;
  if (organizations && !organizations.data)
    error(
      organizations.response.status,
      organizations.error?.message ?? 'Organizations unavailable',
    );
  const tags = await tagPanel(
    client,
    workspace.id,
    result.data.id,
    'contacts',
    event.url.searchParams.get('tag_search') ?? '',
  );
  return {
    tags,
    workspace,
    contact: result.data,
    relationships: relationships.data ?? [],
    canReadOrganizations: !!relationships.data,
    organizations: organizations?.data?.items ?? [],
    organizationSearch,
    relationshipKey: randomUUID(),
    editKey: randomUUID(),
    archiveKey: randomUUID(),
  };
};
export const actions: Actions = {
  tags: (event) =>
    saveTags(event, event.params.workspace, event.params.contact, 'contacts'),
  relate: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.PUT(
      '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/organizations/{organization_id}',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            contact_id: event.params.contact,
            organization_id: String(form.get('organization_id')),
          },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          expected_contact_version: Number(
            form.get('expected_contact_version'),
          ),
          relationship_label: String(form.get('relationship_label') ?? ''),
          job_title: String(form.get('job_title') ?? ''),
          is_primary: form.get('is_primary') === 'on',
          ended: form.get('ended') === 'true',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message:
          result.error?.message ?? 'Could not change organization relationship',
      });
    redirect(303, event.url.pathname);
  },
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

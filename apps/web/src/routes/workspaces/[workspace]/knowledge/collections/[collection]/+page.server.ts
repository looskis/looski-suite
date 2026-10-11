import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey, text, version } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = {
    workspace_id: workspace.id,
    collection_id: event.params.collection,
  };
  const collection = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}',
    { params: { path } },
  );
  if (!collection.data)
    error(
      collection.response.status,
      collection.error?.message ?? 'Collection unavailable',
    );
  const archived = event.url.searchParams.get('archived') === 'true';
  const documents = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/documents',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          collection_id: collection.data.id,
          archived,
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!documents.data)
    error(
      documents.response.status,
      documents.error?.message ?? 'Documents unavailable',
    );
  const members = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}/members',
    { params: { path } },
  );
  // Only members who can manage collections see the directory.
  const directory = await client.GET(
    '/api/v1/workspaces/{workspace_id}/knowledge/directory',
    { params: { path: { workspace_id: workspace.id } } },
  );
  const listed = new Set((members.data ?? []).map((m) => m.id));
  return {
    workspace,
    collection: collection.data,
    documents: documents.data,
    archived,
    members: members.data ?? [],
    canManage: Boolean(directory.data),
    candidates: (directory.data ?? []).filter((m) => !listed.has(m.id)),
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
    const input = {
      name: text(form, 'name'),
      description: text(form, 'description'),
      restricted: form.get('restricted') === 'on',
      expected_version: version(form),
    };
    const key = retryKey(form);
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            collection_id: event.params.collection,
          },
          header: { 'Idempotency-Key': key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save the collection',
        input,
        key,
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
      '/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}/archive',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            collection_id: event.params.collection,
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: {
          expected_version: version(form) ?? 0,
          archived: form.get('archived') === 'true',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change the collection',
      });
    redirect(303, event.url.pathname);
  },
  add: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}/members',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            collection_id: event.params.collection,
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: { membership_id: String(form.get('membership_id') ?? '') },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not add the member',
      });
    redirect(303, event.url.pathname);
  },
  remove: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}/members/{membership_id}/remove',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            collection_id: event.params.collection,
            membership_id: String(form.get('membership_id') ?? ''),
          },
          header: { 'Idempotency-Key': retryKey(form) },
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not remove the member',
      });
    // Removing yourself from a restricted collection takes you out of it.
    const still = await client.GET(
      '/api/v1/workspaces/{workspace_id}/knowledge/collections/{collection_id}',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            collection_id: event.params.collection,
          },
        },
      },
    );
    redirect(
      303,
      still.data ? event.url.pathname : `/workspaces/${workspace.id}/knowledge`,
    );
  },
};

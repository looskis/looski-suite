import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey, text } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, id: event.params.conversation };
  const conversation = await client.GET(
    '/api/v1/workspaces/{workspace_id}/communications/conversations/{id}',
    { params: { path } },
  );
  if (!conversation.data)
    error(
      conversation.response.status,
      conversation.error?.message ?? 'Conversation unavailable',
    );
  const messages = await client.GET(
    '/api/v1/workspaces/{workspace_id}/communications/conversations/{id}/messages',
    {
      params: {
        path,
        query: {
          after: event.url.searchParams.get('after') || undefined,
          limit: 50,
          newest_first: true,
        },
      },
    },
  );
  if (!messages.data)
    error(
      messages.response.status,
      messages.error?.message ?? 'Messages unavailable',
    );
  const accounts = await client.GET(
    '/api/v1/workspaces/{workspace_id}/communications/accounts',
    { params: { path } },
  );
  const search = event.url.searchParams.get('search') ?? '';
  const contacts = await client.GET(
    '/api/v1/workspaces/{workspace_id}/contacts',
    { params: { path, query: { search: search || undefined, limit: 30 } } },
  );
  const contact = conversation.data.contact_id
    ? await client.GET(
        '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}',
        {
          params: {
            path: {
              workspace_id: workspace.id,
              contact_id: conversation.data.contact_id,
            },
          },
        },
      )
    : null;
  return {
    workspace,
    olderPage: event.url.searchParams.has('after'),
    conversation: conversation.data,
    messages: { ...messages.data, items: [...messages.data.items].reverse() },
    account:
      accounts.data?.find((a) => a.id === conversation.data.account_id) ?? null,
    contacts: contacts.data?.items ?? [],
    contact: contact?.data ?? null,
    search,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  send: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const key = retryKey(f);
    const body = text(f, 'body');
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/communications/conversations/{id}/messages',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.conversation },
          header: { 'Idempotency-Key': key },
        },
        body: { body },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not queue reply',
        body,
        key,
      });
    redirect(303, event.url.pathname);
  },
  update: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/communications/conversations/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.conversation },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          expected_version: Number(f.get('expected_version')),
          contact_id: text(f, 'contact_id') || null,
          archived: f.get('archived') === 'on',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save conversation',
      });
    redirect(303, event.url.pathname);
  },
};

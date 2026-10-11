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
  const path = { workspace_id: workspace.id, ticket_id: event.params.ticket };
  const ticket = await client.GET(
    '/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}',
    { params: { path } },
  );
  if (!ticket.data)
    error(
      ticket.response.status,
      ticket.error?.message ?? 'Ticket unavailable',
    );
  const messages = await client.GET(
    '/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}/messages',
    {
      params: {
        path,
        query: {
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!messages.data)
    error(
      messages.response.status,
      messages.error?.message ?? 'Messages unavailable',
    );
  const directory = await client.GET(
    '/api/v1/workspaces/{workspace_id}/helpdesk/directory',
    { params: { path } },
  );
  const contactSearch = event.url.searchParams.get('contact_search') ?? '';
  const contacts = await client.GET(
    '/api/v1/workspaces/{workspace_id}/contacts',
    {
      params: {
        path,
        query: { search: contactSearch || undefined, limit: 30 },
      },
    },
  );
  const contact = ticket.data.contact_id
    ? await client.GET(
        '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}',
        { params: { path: { ...path, contact_id: ticket.data.contact_id } } },
      )
    : null;
  const q = event.url.searchParams.get('q') ?? '';
  const articles = q
    ? await client.GET('/api/v1/workspaces/{workspace_id}/knowledge/suggest', {
        params: { path, query: { q, limit: 20 } },
      })
    : null;
  return {
    workspace,
    contacts: contacts.data?.items ?? [],
    contact: contact?.data ?? null,
    contactSearch,
    ticket: ticket.data,
    messages: messages.data,
    directory: directory.data ?? [],
    key: randomUUID(),
    q,
    articles: articles?.data?.items ?? [],
  };
};
export const actions: Actions = {
  reply: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const key = retryKey(f);
    const input = {
      body: text(f, 'body'),
      internal: f.get('internal') === 'true',
    };
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}/messages',
      {
        params: {
          path: { workspace_id: workspace.id, ticket_id: event.params.ticket },
          header: { 'Idempotency-Key': key },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not send message',
        input,
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
      '/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}',
      {
        params: {
          path: { workspace_id: workspace.id, ticket_id: event.params.ticket },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          expected_version: Number(f.get('expected_version')),
          status: text(f, 'status'),
          assignee_id: text(f, 'assignee_id') || null,
          archived: f.get('archived') === 'true',
          revoke_customer_access: f.get('revoke') === 'on',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not update ticket',
      });
    redirect(303, event.url.pathname);
  },
  links: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const result = await client.PUT(
      '/api/v1/workspaces/{workspace_id}/helpdesk/tickets/{ticket_id}/links',
      {
        params: {
          path: { workspace_id: workspace.id, ticket_id: event.params.ticket },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          expected_version: Number(f.get('expected_version')),
          contact_id: text(f, 'contact_id') || null,
          article_ids: f.getAll('article_ids').map(String),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save links',
      });
    redirect(303, event.url.pathname);
  },
};

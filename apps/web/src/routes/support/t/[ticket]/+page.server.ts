import { randomUUID } from 'node:crypto';
import { error, fail, redirect } from '@sveltejs/kit';
import { publicClient } from '$lib/server/public-form';
import { requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const code = event.cookies.get('looski_ticket');
  const base = { key: randomUUID() };
  if (!code) return { ...base, ticket: null, messages: null };
  const params = {
    path: { ticket_id: event.params.ticket },
    header: { 'X-Ticket-Key': code },
  };
  const client = publicClient(event);
  const ticket = await client.GET(
    '/api/v1/public/helpdesk/tickets/{ticket_id}',
    { params },
  );
  if (!ticket.data) {
    if (ticket.response.status === 404)
      return { ...base, ticket: null, messages: null };
    error(
      ticket.response.status,
      'Ticket temporarily unavailable. Please retry.',
    );
  }
  const messages = await client.GET(
    '/api/v1/public/helpdesk/tickets/{ticket_id}/messages',
    {
      params: {
        ...params,
        query: {
          after: event.url.searchParams.get('after') ?? undefined,
          limit: 50,
        },
      },
    },
  );
  if (!messages.data) error(messages.response.status, 'Messages unavailable');
  return { ...base, ticket: ticket.data, messages: messages.data };
};
export const actions: Actions = {
  unlock: async (event) => {
    requireSameOrigin(event);
    const f = await event.request.formData();
    const code = String(f.get('code') ?? '').trim();
    const result = await publicClient(event).GET(
      '/api/v1/public/helpdesk/tickets/{ticket_id}',
      {
        params: {
          path: { ticket_id: event.params.ticket },
          header: { 'X-Ticket-Key': code },
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message:
          result.response.status === 404
            ? 'Ticket or access code unavailable. Contact the support team if access was revoked.'
            : 'Please try again shortly.',
      });
    event.cookies.set('looski_ticket', code, {
      path: event.url.pathname,
      httpOnly: true,
      sameSite: 'strict',
      secure: event.url.protocol === 'https:',
      maxAge: 60 * 60 * 24 * 30,
    });
    redirect(303, event.url.pathname);
  },
  reply: async (event) => {
    requireSameOrigin(event);
    const f = await event.request.formData();
    const body = String(f.get('body') ?? '');
    const key = String(f.get('idempotency_key') ?? '');
    const result = await publicClient(event).POST(
      '/api/v1/public/helpdesk/tickets/{ticket_id}/messages',
      {
        params: {
          path: { ticket_id: event.params.ticket },
          header: {
            'X-Ticket-Key': event.cookies.get('looski_ticket') ?? '',
            'Idempotency-Key': key,
          },
        },
        body: { body },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not send your reply',
        body,
        key,
      });
    redirect(303, event.url.pathname);
  },
  lock: async (event) => {
    requireSameOrigin(event);
    event.cookies.delete('looski_ticket', { path: event.url.pathname });
    redirect(303, event.url.pathname);
  },
};

import { randomBytes } from 'node:crypto';
import { error, fail } from '@sveltejs/kit';
import { publicClient } from '$lib/server/public-form';
import { requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const result = await publicClient(event).GET(
    '/api/v1/public/helpdesk/intake/{public_key}',
    { params: { path: { public_key: event.params.public_key } } },
  );
  if (!result.data && event.request.method !== 'POST')
    error(
      result.response.status,
      'This support page is not accepting new tickets.',
    );
  return { code: randomBytes(32).toString('hex') };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const f = await event.request.formData();
    const code = String(f.get('code') ?? '');
    const input = {
      subject: String(f.get('subject') ?? ''),
      requester_name: String(f.get('requester_name') ?? ''),
      body: String(f.get('body') ?? ''),
    };
    const result = await publicClient(event).POST(
      '/api/v1/public/helpdesk/intake/{public_key}',
      {
        params: {
          path: { public_key: event.params.public_key },
          header: { 'X-Ticket-Key': code },
        },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not open ticket',
        input,
        code,
      });
    const path = `/support/t/${result.data.id}`;
    event.cookies.set('looski_ticket', code, {
      path,
      httpOnly: true,
      sameSite: 'strict',
      secure: event.url.protocol === 'https:',
      maxAge: 60 * 60 * 24 * 30,
    });
    return { created: true, ticketPath: path, code };
  },
};

import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { identityClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
import { cookie, invitation, captureInvitation } from '$lib/server/invitation';
export const load: PageServerLoad = async (event) => {
  if (event.url.search) {
    if (!captureInvitation(event, event.url))
      error(400, 'This invitation link is invalid.');
    redirect(303, '/invitations/accept');
  }
  const pending = invitation(event.cookies.get(cookie));
  if (!pending)
    return {
      pending: false,
      preview: null,
      message: null,
      signedIn: false,
      key: randomUUID(),
    };
  if (!event.cookies.get('looski_refresh'))
    return {
      pending: true,
      preview: null,
      message: null,
      signedIn: false,
      key: randomUUID(),
    };
  const client = await identityClient(event);
  const result = await client.POST(
    '/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}/preview',
    {
      params: {
        path: { workspace_id: pending.workspace, invitation_id: pending.id },
      },
      body: { token: pending.token },
    },
  );
  return {
    pending: true,
    preview: result.data ?? null,
    message: result.error?.message ?? null,
    signedIn: true,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const pending = invitation(event.cookies.get(cookie));
    if (!pending)
      return fail(400, {
        message: 'Open your invitation link first.',
        key: randomUUID(),
        display_name: '',
      });
    const client = await identityClient(event);
    const form = await event.request.formData();
    const display_name = String(form.get('display_name') ?? '');
    const key = String(form.get('idempotency_key') ?? '');
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/access/invitations/{invitation_id}/accept',
      {
        params: {
          path: { workspace_id: pending.workspace, invitation_id: pending.id },
          header: { 'Idempotency-Key': key },
        },
        body: { token: pending.token, display_name },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not accept invitation',
        display_name,
        key,
      });
    event.cookies.delete(cookie, { path: '/' });
    redirect(303, '/workspaces');
  },
};

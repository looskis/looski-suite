import { fail, redirect } from '@sveltejs/kit';
import {
  authRequest,
  requireSameOrigin,
  saveSession,
} from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';

import {
  pendingCookie,
  confirmationPath as cookiePath,
  pendingEmailLink as pending,
  captureEmailLink,
} from '$lib/server/email-link';

export const load: PageServerLoad = (event) => {
  if (event.url.search) {
    const token = event.url.searchParams.get('token_hash');
    const kind = event.url.searchParams.get('type');
    captureEmailLink(event, token, kind);
  }
  return { kind: pending(event.cookies.get(pendingCookie))?.kind };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const link = pending(event.cookies.get(pendingCookie));
    if (!link)
      return fail(400, {
        message: 'This email link is invalid or expired. Request a new link.',
      });
    let response;
    try {
      response = await authRequest(event, '/verify', {
        token_hash: link.token,
        type: link.kind,
      });
    } catch {
      return fail(503, {
        message: 'Sign-in is temporarily unavailable. Please try again.',
      });
    }
    if (!response.ok) {
      if (response.status >= 500 || response.status === 429)
        return fail(503, {
          message:
            'Sign-in is temporarily unavailable. Please try again later.',
        });
      event.cookies.delete(pendingCookie, { path: cookiePath });
      return fail(400, {
        message:
          'This email link is invalid, expired or already used. Request a new link.',
      });
    }
    saveSession(event, await response.json());
    event.cookies.delete(pendingCookie, { path: cookiePath });
    redirect(303, link.kind === 'recovery' ? '/auth/password' : '/workspaces');
  },
};

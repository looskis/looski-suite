import { fail, redirect } from '@sveltejs/kit';
import {
  authRequest,
  clearSession,
  requireSameOrigin,
  saveSession,
} from '$lib/server/session';
import type { Actions, PageServerLoad, RequestEvent } from './$types';

async function sessionToken(event: RequestEvent) {
  const refresh = event.cookies.get('looski_refresh');
  if (!refresh) redirect(303, '/login');
  let response;
  try {
    response = await authRequest(event, '/token?grant_type=refresh_token', {
      refresh_token: refresh,
    });
  } catch {
    return;
  }
  if (!response.ok) {
    if (response.status >= 500 || response.status === 429) return;
    clearSession(event);
    redirect(303, '/login');
  }
  const session = await response.json();
  saveSession(event, session);
  return session.access_token as string;
}
export const load: PageServerLoad = (event) => {
  if (!event.cookies.get('looski_refresh')) redirect(303, '/login');
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const form = await event.request.formData();
    const password = form.get('password');
    if (
      typeof password !== 'string' ||
      password.length < 12 ||
      password.length > 1024 ||
      password !== form.get('confirm')
    )
      return fail(400, {
        message: 'Enter matching passwords of 12 to 1024 characters.',
      });
    const token = await sessionToken(event);
    if (!token)
      return fail(503, {
        message: 'The service is temporarily unavailable. Please try again.',
      });
    try {
      const response = await authRequest(
        event,
        '/user',
        { password },
        token,
        'PUT',
      );
      if (!response.ok)
        return fail(response.status >= 500 ? 503 : 400, {
          message:
            'Could not update your password. Try a different password or request a new recovery link.',
        });
    } catch {
      return fail(503, {
        message: 'The service is temporarily unavailable. Please try again.',
      });
    }
    redirect(303, '/workspaces');
  },
};

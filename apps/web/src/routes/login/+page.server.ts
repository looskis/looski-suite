import { fail, redirect } from '@sveltejs/kit';
import {
  authRequest,
  requireSameOrigin,
  saveSession,
} from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = ({ url }) => ({
  localLogout: url.searchParams.get('logout') === 'local',
});

export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const form = await event.request.formData();
    const email = form.get('email');
    const password = form.get('password');
    if (
      typeof email !== 'string' ||
      typeof password !== 'string' ||
      email.length > 254 ||
      password.length > 1024 ||
      !email ||
      !password
    ) {
      return fail(400, { message: 'Enter your email and password.' });
    }
    let response;
    try {
      response = await authRequest(event, '/token?grant_type=password', {
        email,
        password,
      });
    } catch {
      return fail(503, {
        message: 'Sign-in is temporarily unavailable. Please try again.',
      });
    }
    if (response.status === 429)
      return fail(429, {
        message: 'Too many attempts. Please wait before trying again.',
      });
    if (!response.ok)
      return fail(response.status >= 500 ? 503 : 400, {
        message: 'Could not sign in. Check your credentials and try again.',
      });
    saveSession(event, await response.json());
    redirect(303, '/workspaces');
  },
};

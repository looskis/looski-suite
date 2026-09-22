import { env } from '$env/dynamic/private';
import { fail } from '@sveltejs/kit';
import { authRequest, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
  enabled: env.LOOSKI_AUTH_MAIL_ENABLED === 'true',
});
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    if (env.LOOSKI_AUTH_MAIL_ENABLED !== 'true')
      return fail(503, {
        message: 'Email sign-in is not configured for this installation.',
      });
    const form = await event.request.formData();
    const email = form.get('email');
    const kind = form.get('kind');
    if (
      typeof email !== 'string' ||
      !email ||
      email.length > 254 ||
      !['magiclink', 'recovery'].includes(String(kind))
    )
      return fail(400, {
        message: 'Enter your email and choose a sign-in method.',
      });
    try {
      const response = await authRequest(
        event,
        kind === 'magiclink' ? '/otp' : '/recover',
        { email, create_user: false },
      );
      if (response.status >= 500)
        console.warn('Authentication email delivery failed', {
          status: response.status,
        });
    } catch {
      console.warn('Authentication email delivery unavailable');
    }
    // Keep missing accounts, delivery outages and throttling indistinguishable.
    // Diagnostics never contain the email, provider body, link or SMTP credentials.
    return {
      message:
        'If this address has an eligible account, an email will arrive shortly. Check your inbox before requesting another link. If no message arrives, contact your installation administrator.',
    };
  },
};

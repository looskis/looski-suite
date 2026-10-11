import type { Handle } from '@sveltejs/kit';

const publicFormEmbed = /^\/f\/[A-Za-z0-9_-]{22}\/embed$/;
// Published pages ship no scripts, so a policy can forbid them outright: a second
// wall behind typed rendering. Their only frame is a same-origin form embed.
const publicContent =
  "default-src 'none'; img-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; frame-src 'self'; base-uri 'none'; form-action 'none'";

export const handle: Handle = async ({ event, resolve }) => {
  const response = await resolve(event);
  response.headers.set('Cache-Control', 'no-store');
  // A redirect may carry a token in the URL it leaves. Pages must stay same-origin:
  // under no-referrer, browsers send `Origin: null` on form posts, which CSRF refuses.
  const redirect = response.status >= 300 && response.status < 400;
  response.headers.set(
    'Referrer-Policy',
    redirect &&
      (event.url.pathname.startsWith('/auth/') ||
        event.url.pathname.startsWith('/invitations/') ||
        event.url.pathname.startsWith('/cli/'))
      ? 'no-referrer'
      : 'same-origin',
  );
  // Only the embed variant may be framed by other sites. The form it serves
  // needs no cookie and posts to its own origin, so a frame changes no CSRF rule.
  const policy = event.url.pathname.startsWith('/c/')
    ? publicContent
    : response.headers.get('Content-Security-Policy');
  if (publicFormEmbed.test(event.url.pathname)) {
    response.headers.set('Content-Security-Policy', 'frame-ancestors *');
  } else {
    response.headers.set(
      'Content-Security-Policy',
      policy ? `${policy}; frame-ancestors 'none'` : "frame-ancestors 'none'",
    );
    response.headers.set('X-Frame-Options', 'DENY');
  }
  return response;
};

import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  const response = await resolve(event);
  response.headers.set('Cache-Control', 'no-store');
  response.headers.set(
    'Referrer-Policy',
    event.url.pathname.startsWith('/auth/') ||
      event.url.pathname.startsWith('/invitations/')
      ? 'no-referrer'
      : 'same-origin',
  );
  return response;
};

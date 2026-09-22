import { error, redirect, type RequestEvent } from '@sveltejs/kit';

import { captureInvitation } from '$lib/server/invitation';

export const pendingCookie = 'looski_pending_link';
export const confirmationPath = '/auth/confirm';
export function validEmailLink(token: unknown, kind: unknown) {
  return (
    typeof token === 'string' &&
    /^[a-f0-9]{56,128}$/.test(token) &&
    (kind === 'magiclink' || kind === 'recovery')
  );
}
export function pendingEmailLink(
  value: string | undefined,
): { token: string; kind: string } | undefined {
  if (!value) return;
  const [kind, token] = value.split(':');
  if (validEmailLink(token, kind)) return { token, kind };
}
export function captureEmailLink(
  event: RequestEvent,
  token: string | null,
  kind: string | null,
): never {
  if (!validEmailLink(token, kind))
    error(400, 'This email link is invalid. Request a new link.');
  const target = event.url.searchParams.get('redirect_to');
  if (target) {
    try {
      captureInvitation(event, new URL(target));
    } catch {
      /* Ignore malformed return targets. */
    }
  }
  event.cookies.set(pendingCookie, `${kind}:${token}`, {
    path: confirmationPath,
    httpOnly: true,
    secure: event.url.protocol === 'https:',
    sameSite: 'lax',
    maxAge: 600,
  });
  redirect(303, confirmationPath);
}

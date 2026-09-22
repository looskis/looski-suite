import type { RequestEvent } from '@sveltejs/kit';
export const cookie = 'looski_pending_invitation';
export function invitation(
  value: string | undefined,
): { workspace: string; id: string; token: string } | undefined {
  if (!value || value.length > 2600) return;
  try {
    const item = JSON.parse(value);
    if (
      typeof item.workspace === 'string' &&
      /^[-a-f0-9]{36}$/.test(item.workspace) &&
      typeof item.id === 'string' &&
      /^[-a-f0-9]{36}$/.test(item.id) &&
      typeof item.token === 'string' &&
      item.token.length <= 2048 &&
      /^[A-Za-z0-9_.-]+$/.test(item.token)
    )
      return item;
  } catch {
    /* Invalid pending cookies never become an application identity. */
  }
}

export function captureInvitation(event: RequestEvent, target: URL): boolean {
  if (
    target.origin !== event.url.origin ||
    target.pathname !== '/invitations/accept' ||
    target.username ||
    target.password
  )
    return false;
  const value = JSON.stringify({
    workspace: target.searchParams.get('workspace'),
    id: target.searchParams.get('invitation'),
    token: target.searchParams.get('token'),
  });
  if (!invitation(value)) return false;
  event.cookies.set(cookie, value, {
    path: '/',
    httpOnly: true,
    secure: event.url.protocol === 'https:',
    sameSite: 'lax',
    maxAge: 7 * 86400,
  });
  return true;
}

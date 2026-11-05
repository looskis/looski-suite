import { createHash, randomBytes } from 'node:crypto';
import { env } from '$env/dynamic/private';
import { error, redirect, type RequestEvent } from '@sveltejs/kit';
import { authRequest, requireSameOrigin, saveSession } from './session';
import { safeReturnPath } from './return-path';

export const googleEnabled = () => env.LOOSKI_GOOGLE_AUTH_ENABLED === 'true';
const lifetime = 600;
const callbackPath = '/auth/callback';

function cookieName(event: RequestEvent) {
  return event.url.protocol === 'https:'
    ? '__Host-looski_google'
    : 'looski_google';
}

type Pending = { verifier: string; expires: number; next?: string };

function readPending(value: string | undefined): Pending | undefined {
  if (!value || value.length > 4096) return;
  try {
    const pending = JSON.parse(Buffer.from(value, 'base64url').toString());
    if (
      typeof pending.verifier !== 'string' ||
      !/^[A-Za-z0-9_-]{43}$/.test(pending.verifier) ||
      !Number.isFinite(pending.expires) ||
      pending.expires <= Date.now() ||
      pending.expires > Date.now() + lifetime * 1000 ||
      (pending.next !== undefined && typeof pending.next !== 'string')
    )
      return;
    return pending;
  } catch {
    return;
  }
}

export async function startGoogle(event: RequestEvent) {
  requireSameOrigin(event);
  if (!googleEnabled()) error(404, 'Google sign-in is not enabled.');
  const origin = env.LOOSKI_PUBLIC_URL;
  if (!origin || origin !== event.url.origin)
    error(503, 'Google sign-in is temporarily unavailable.');
  const next = safeReturnPath(event.url.searchParams.get('next'), origin);
  const verifier = randomBytes(32).toString('base64url');
  const authorize = new URL(
    `${env.LOOSKI_AUTH_INTERNAL_URL ?? 'http://127.0.0.1:9999'}/authorize`,
  );
  authorize.search = new URLSearchParams({
    provider: 'google',
    prompt: 'select_account',
    redirect_to: origin + callbackPath,
    code_challenge: createHash('sha256').update(verifier).digest('base64url'),
    code_challenge_method: 's256',
  }).toString();
  let location: string;
  try {
    // Auth owns Google's state and token exchange. Only its provider redirect is
    // returned to the browser; /authorize and /token remain private.
    const response = await event.fetch(authorize, {
      redirect: 'manual',
      signal: AbortSignal.timeout(5000),
    });
    const target = new URL(response.headers.get('location') ?? '');
    if (
      ![302, 303].includes(response.status) ||
      target.origin !== 'https://accounts.google.com' ||
      target.username ||
      target.password
    )
      throw new Error('Invalid provider redirect');
    location = target.href;
  } catch {
    error(503, 'Google sign-in is temporarily unavailable. Please try again.');
  }
  event.cookies.set(
    cookieName(event),
    Buffer.from(
      JSON.stringify({ verifier, next, expires: Date.now() + lifetime * 1000 }),
    ).toString('base64url'),
    {
      path: '/',
      httpOnly: true,
      secure: event.url.protocol === 'https:',
      sameSite: 'lax',
      maxAge: lifetime,
    },
  );
  redirect(303, location);
}

export async function finishGoogle(event: RequestEvent) {
  const pending = readPending(event.cookies.get(cookieName(event)));
  event.cookies.delete(cookieName(event), { path: '/' });
  const code = event.url.searchParams.get('code');
  const next = safeReturnPath(pending?.next, event.url.origin);
  const failure =
    '/login?' +
    new URLSearchParams({ google: 'failed', ...(next ? { next } : {}) }) +
    '#'; // Clear Google's error fragment rather than inheriting it on redirect.
  if (
    !googleEnabled() ||
    !pending ||
    !code ||
    code.length > 2048 ||
    event.url.searchParams.has('error')
  )
    redirect(303, failure);
  let session;
  try {
    const response = await authRequest(event, '/token?grant_type=pkce', {
      auth_code: code,
      code_verifier: pending.verifier,
    });
    if (response.ok) session = await response.json();
  } catch {
    // Never log codes, provider responses, or credentials.
  }
  if (
    !session ||
    typeof session.access_token !== 'string' ||
    !session.access_token ||
    typeof session.refresh_token !== 'string' ||
    !session.refresh_token ||
    !Number.isFinite(session.expires_in) ||
    session.expires_in <= 0
  )
    redirect(303, failure);
  saveSession(event, session);
  redirect(303, next ?? '/workspaces');
}

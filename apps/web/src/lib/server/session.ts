import { env } from '$env/dynamic/private';
import {
  error,
  redirect,
  isRedirect,
  isHttpError,
  type RequestEvent,
} from '@sveltejs/kit';
import createClient from 'openapi-fetch';
import type { paths } from '$lib/api/schema';
import { loginPath } from '$lib/server/return-path';

const accessCookie = 'looski_access';
const refreshCookie = 'looski_refresh';

type Session = {
  access_token: string;
  refresh_token: string;
  expires_in: number;
};

export async function authRequest(
  event: RequestEvent,
  path: string,
  body?: unknown,
  token?: string,
  method: 'GET' | 'POST' | 'PUT' = 'POST',
) {
  return event.fetch(
    `${env.LOOSKI_AUTH_INTERNAL_URL ?? 'http://127.0.0.1:9999'}${path}`,
    {
      method,
      headers: {
        'Content-Type': 'application/json',
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(5000),
      redirect: 'error',
    },
  );
}

export function requireSameOrigin(event: RequestEvent) {
  if (event.request.headers.get('origin') !== event.url.origin)
    error(403, 'Invalid request origin');
}

export function clearSession(event: RequestEvent) {
  event.cookies.delete(accessCookie, { path: '/' });
  event.cookies.delete(refreshCookie, { path: '/' });
}

export function saveSession(event: RequestEvent, session: Session) {
  if (
    !session.access_token ||
    !session.refresh_token ||
    !Number.isFinite(session.expires_in)
  ) {
    error(503, 'Sign-in is temporarily unavailable');
  }
  const options = {
    path: '/',
    httpOnly: true,
    secure: event.url.protocol === 'https:',
    sameSite: 'lax' as const,
  };
  event.cookies.set(accessCookie, session.access_token, {
    ...options,
    maxAge: session.expires_in,
  });
  event.cookies.set(refreshCookie, session.refresh_token, {
    ...options,
    maxAge: 60 * 60 * 24 * 30,
  });
}

/** `returnTo` is an already validated same-origin path to resume after sign-in. */
export async function workspaceList(event: RequestEvent, returnTo?: string) {
  const client = createClient<paths>({
    baseUrl: env.LOOSKI_API_INTERNAL_URL ?? 'http://127.0.0.1:58081',
    fetch: event.fetch,
  });
  const read = (token: string) =>
    client.GET('/api/v1/workspaces', {
      headers: {
        Authorization: `Bearer ${token}`,
        'X-Looski-Interface': 'web',
      },
      signal: AbortSignal.timeout(5000),
      redirect: 'error',
    });
  let token = event.cookies.get(accessCookie);
  let result;
  try {
    result = token ? await read(token) : undefined;
    if (!result || result.response.status === 401) {
      const refresh = event.cookies.get(refreshCookie);
      if (!refresh) {
        clearSession(event);
        redirect(303, loginPath(returnTo));
      }
      const response = await authRequest(
        event,
        '/token?grant_type=refresh_token',
        { refresh_token: refresh },
      );
      if (
        response.status === 400 ||
        response.status === 401 ||
        response.status === 403
      ) {
        clearSession(event);
        redirect(303, loginPath(returnTo));
      }
      if (!response.ok) error(503, 'Sign-in is temporarily unavailable');
      const session: Session = await response.json();
      saveSession(event, session);
      token = session.access_token;
      result = await read(token);
    }
  } catch (failure) {
    // Preserve SvelteKit redirects and HTTP errors; never expose upstream errors/tokens.
    if (isRedirect(failure) || isHttpError(failure)) throw failure;
    error(503, 'The service is temporarily unavailable. Please try again.');
  }
  if (result.response.status === 401) {
    clearSession(event);
    redirect(303, loginPath(returnTo));
  }
  if (!result.data) error(503, 'Workspaces are temporarily unavailable');
  return result.data;
}

export async function signOut(event: RequestEvent) {
  let token = event.cookies.get(accessCookie);
  const refresh = event.cookies.get(refreshCookie);
  let revoked = true;
  try {
    if (refresh) {
      const response = await authRequest(
        event,
        '/token?grant_type=refresh_token',
        { refresh_token: refresh },
      );
      if (response.ok)
        token = ((await response.json()) as Session).access_token;
      else revoked = false;
    }
    if (token) {
      const response = await authRequest(
        event,
        '/logout?scope=local',
        undefined,
        token,
      );
      revoked = revoked && response.ok;
    }
  } catch {
    revoked = false;
  }
  clearSession(event);
  return revoked;
}

/** Refresh through the same verified workspace flow before a business request. */
export async function workspaceClient(event: RequestEvent, workspace: string) {
  const workspaces = await workspaceList(event);
  const selected = workspaces.find((item) => item.id === workspace);
  if (!selected) error(403, 'Workspace access is not permitted');
  const token = event.cookies.get(accessCookie);
  if (!token) redirect(303, '/login');
  return {
    workspace: selected,
    client: createClient<paths>({
      baseUrl: env.LOOSKI_API_INTERNAL_URL ?? 'http://127.0.0.1:58081',
      fetch: event.fetch,
      headers: {
        Authorization: `Bearer ${token}`,
        'X-Looski-Interface': 'web',
      },
    }),
  };
}

/** An authenticated identity may accept an invitation before having a workspace. */
export async function identityClient(event: RequestEvent, returnTo?: string) {
  await workspaceList(event, returnTo);
  const token = event.cookies.get(accessCookie);
  if (!token) redirect(303, loginPath(returnTo));
  return createClient<paths>({
    baseUrl: env.LOOSKI_API_INTERNAL_URL ?? 'http://127.0.0.1:58081',
    fetch: event.fetch,
    headers: { Authorization: `Bearer ${token}`, 'X-Looski-Interface': 'web' },
  });
}

/** The verified browser account's current Auth email, for confirmation screens. */
export async function signedInEmail(event: RequestEvent) {
  const token = event.cookies.get(accessCookie);
  if (!token) return null;
  try {
    const response = await authRequest(event, '/user', undefined, token, 'GET');
    if (!response.ok) return null;
    const user: { email?: unknown } = await response.json();
    return typeof user.email === 'string' ? user.email : null;
  } catch {
    return null;
  }
}

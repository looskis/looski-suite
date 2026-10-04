import { error, fail, redirect, type RequestEvent } from '@sveltejs/kit';
import {
  identityClient,
  requireSameOrigin,
  signedInEmail,
} from '$lib/server/session';
import { loginPath } from '$lib/server/return-path';
import type { Actions, PageServerLoad } from './$types';

type Request = { challenge: string; state: string; device: string | null };

/** The CLI generates these; nothing here is secret, but all of it is untrusted. */
function request(values: {
  get(name: string): FormDataEntryValue | string | null;
}): Request | undefined {
  const challenge = values.get('code_challenge');
  const method = values.get('code_challenge_method');
  const state = values.get('state');
  const device = values.get('device');
  if (
    typeof challenge !== 'string' ||
    !/^[A-Za-z0-9_-]{43}$/.test(challenge) ||
    method !== 'S256' ||
    typeof state !== 'string' ||
    !/^[A-Za-z0-9_-]{16,128}$/.test(state) ||
    (device !== null &&
      (typeof device !== 'string' ||
        !device.trim() ||
        device.length > 64 ||
        /[\u0000-\u001f\u007f]/.test(device)))
  )
    return;
  return { challenge, state, device: device?.trim() || null };
}

function returnPath(item: Request) {
  const query = new URLSearchParams({
    code_challenge: item.challenge,
    code_challenge_method: 'S256',
    state: item.state,
  });
  if (item.device) query.set('device', item.device);
  return `/cli/authorize?${query}`;
}

async function signedIn(event: RequestEvent, item: Request) {
  if (!event.cookies.get('looski_refresh'))
    redirect(303, loginPath(returnPath(item)));
  return identityClient(event, returnPath(item));
}

export const load: PageServerLoad = async (event) => {
  const item = request(event.url.searchParams);
  if (!item)
    error(400, 'This sign-in link is invalid. Run `looski login` again.');
  await signedIn(event, item);
  return {
    challenge: item.challenge,
    state: item.state,
    device: item.device,
    // Named actions keep the request in the URL so this load can rerun.
    query: returnPath(item).split('?')[1],
    origin: event.url.origin,
    email: await signedInEmail(event),
  };
};

export const actions: Actions = {
  approve: async (event) => {
    requireSameOrigin(event);
    const item = request(await event.request.formData());
    if (!item)
      return fail(400, {
        message: 'This sign-in request is invalid. Run `looski login` again.',
      });
    const client = await signedIn(event, item);
    const result = await client.POST('/api/v1/cli/authorizations', {
      body: {
        code_challenge: item.challenge,
        code_challenge_method: 'S256',
        device_label: item.device,
      },
    });
    if (!result.data)
      return fail(result.response.status === 429 ? 429 : 400, {
        message:
          result.error?.message ?? 'Could not authorize the CLI. Try again.',
      });
    // Shown once in this no-store response; never placed in a URL.
    return {
      code: `${result.data.code}#${item.state}`,
      expiresIn: result.data.expires_in,
    };
  },
  cancel: async (event) => {
    requireSameOrigin(event);
    return { cancelled: true };
  },
};

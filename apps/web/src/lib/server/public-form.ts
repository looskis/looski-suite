import { env } from '$env/dynamic/private';
import { error, fail, type RequestEvent } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import createClient from 'openapi-fetch';
import type { paths } from '$lib/api/schema';
import { readAnswers } from '$lib/form-definition';
import { requireSameOrigin } from '$lib/server/session';

type PublicEvent = RequestEvent<{ public_key: string }>;

/**
 * Public pages speak to the API without any credential: they read no session
 * cookie, so a public key can only ever reach its own form. The client address
 * set by the gateway is forwarded so throttles apply per respondent.
 */
export function publicClient(event: RequestEvent) {
  const forwarded = event.request.headers
    .get('x-forwarded-for')
    ?.split(',')
    .at(-1)
    ?.trim();
  return createClient<paths>({
    baseUrl: env.LOOSKI_API_INTERNAL_URL ?? 'http://127.0.0.1:58081',
    fetch: event.fetch,
    headers:
      forwarded && /^[0-9A-Fa-f:.]+$/.test(forwarded)
        ? { 'X-Forwarded-For': forwarded }
        : undefined,
  });
}
async function readForm(event: PublicEvent) {
  try {
    return await publicClient(event).GET('/api/v1/public/forms/{public_key}', {
      params: { path: { public_key: event.params.public_key } },
      signal: AbortSignal.timeout(5000),
    });
  } catch {
    error(503, 'This form is temporarily unavailable. Please try again.');
  }
}
const unavailable = (status: number, message?: string) =>
  status === 404 ? 'This form is not available' : (message ?? 'Please retry');

export async function loadPublicForm(event: PublicEvent) {
  const result = await readForm(event);
  if (!result.data) {
    // A finished or rejected submission still reports its own outcome.
    if (event.request.method === 'POST') return { form: null, key: '' };
    error(
      result.response.status,
      unavailable(result.response.status, result.error?.message),
    );
  }
  return { form: result.data, key: randomUUID() };
}

export async function submitPublicForm(event: PublicEvent) {
  requireSameOrigin(event);
  let data: FormData;
  try {
    data = await event.request.formData();
  } catch {
    error(400, 'This request is not a form submission');
  }
  const key = String(data.get('submission_key') ?? '');
  const form = await readForm(event);
  if (!form.data)
    return fail(form.response.status, {
      message: unavailable(form.response.status, form.error?.message),
      input: {},
      errors: {} as Record<string, string>,
      key,
    });
  const { answers, input, errors } = readAnswers(form.data.definition, data);
  if (Object.keys(errors).length)
    return fail(400, {
      message: 'Check the questions marked below.',
      input,
      errors,
      key,
    });
  try {
    const result = await publicClient(event).POST(
      '/api/v1/public/forms/{public_key}/submissions',
      {
        params: { path: { public_key: event.params.public_key } },
        body: { submission_key: key, answers },
        signal: AbortSignal.timeout(10000),
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: unavailable(result.response.status, result.error?.message),
        input,
        errors,
        // A rejected key can never succeed; every other failure retries safely.
        key: result.response.status === 409 ? randomUUID() : key,
      });
    return {
      confirmation:
        result.data.confirmation_message ||
        'Thank you. Your response was recorded.',
    };
  } catch {
    return fail(503, {
      message: 'Your response could not be sent. Please try again.',
      input,
      errors,
      key,
    });
  }
}

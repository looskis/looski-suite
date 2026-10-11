import { error, type RequestEvent } from '@sveltejs/kit';
import { publicClient } from '$lib/server/public-form';

const unavailable = 'This page is temporarily unavailable. Please try again.';

/** Published reads carry no credential: the type's public key is the only authority. */
export async function loadPublicIndex(
  event: RequestEvent<{ public_key: string }>,
) {
  const result = await publicClient(event)
    .GET('/api/v1/public/cms/{public_key}', {
      params: {
        path: { public_key: event.params.public_key },
        query: { after: event.url.searchParams.get('after') ?? undefined },
      },
      signal: AbortSignal.timeout(5000),
    })
    .catch(() => error(503, unavailable));
  if (!result.data)
    error(
      result.response.status,
      result.response.status === 404
        ? 'This page does not exist'
        : (result.error?.message ?? unavailable),
    );
  return { index: result.data, key: event.params.public_key };
}

export async function loadPublicEntry(
  event: RequestEvent<{ public_key: string; slug: string }>,
) {
  const result = await publicClient(event)
    .GET('/api/v1/public/cms/{public_key}/entries/{slug}', {
      params: {
        path: { public_key: event.params.public_key, slug: event.params.slug },
      },
      signal: AbortSignal.timeout(5000),
    })
    .catch(() => error(503, unavailable));
  if (!result.data)
    error(
      result.response.status,
      result.response.status === 404
        ? 'This page does not exist'
        : (result.error?.message ?? unavailable),
    );
  return { page: result.data, key: event.params.public_key };
}

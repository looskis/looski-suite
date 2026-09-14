import { env } from '$env/dynamic/private';
import createClient from 'openapi-fetch';
import type { paths } from '$lib/api/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const client = createClient<paths>({
    baseUrl: env.LOOSKI_API_INTERNAL_URL ?? 'http://127.0.0.1:58081',
    fetch,
  });
  try {
    const { data, error } = await client.GET('/api/v1/system', {
      signal: AbortSignal.timeout(3000),
    });
    return { system: error ? null : (data ?? null) };
  } catch {
    return { system: null };
  }
};

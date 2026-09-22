import { captureEmailLink } from '$lib/server/email-link';
import type { RequestHandler } from './$types';

// Compatibility with Supabase's default template if the custom template server
// is unavailable. Never proxy verification to Auth or consume a token on GET.
export const GET: RequestHandler = (event) =>
  captureEmailLink(
    event,
    event.url.searchParams.get('token'),
    event.url.searchParams.get('type'),
  );

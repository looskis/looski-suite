import { redirect } from '@sveltejs/kit';
import { requireSameOrigin, signOut } from '$lib/server/session';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async (event) => {
  requireSameOrigin(event);
  const revoked = await signOut(event);
  redirect(303, revoked ? '/login' : '/login?logout=local');
};

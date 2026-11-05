import { workspaceList, signedInEmail } from '$lib/server/session';
import { readAppearance } from '$lib/server/appearance';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const workspaces = await workspaceList(event);
  return {
    pendingInvitation: Boolean(event.cookies.get('looski_pending_invitation')),
    workspaces,
    accountEmail: await signedInEmail(event),
    appearance: readAppearance(event.cookies),
  };
};

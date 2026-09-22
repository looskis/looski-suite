import { workspaceList } from '$lib/server/session';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => ({
  pendingInvitation: Boolean(event.cookies.get('looski_pending_invitation')),
  workspaces: await workspaceList(event),
});

import { workspaceList } from '$lib/server/session';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => ({
  workspaces: await workspaceList(event),
});

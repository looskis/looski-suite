import { workspaceClient, signedInEmail } from '$lib/server/session';
import { readAppearance } from '$lib/server/appearance';
import type { LayoutServerLoad } from './$types';
export const load: LayoutServerLoad = async (event) => {
  const { workspace } = await workspaceClient(event, event.params.workspace);
  return {
    workspace,
    accountEmail: await signedInEmail(event),
    appearance: readAppearance(event.cookies),
  };
};

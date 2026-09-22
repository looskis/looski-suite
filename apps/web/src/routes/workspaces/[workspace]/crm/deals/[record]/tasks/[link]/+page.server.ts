import { loadTaskLinkHistory } from '$lib/server/crm-task-links';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = (event) =>
  loadTaskLinkHistory(
    event,
    event.params.workspace,
    event.params.record,
    event.params.link,
    true,
  );

import {
  loadTaskLinks,
  addTaskLink,
  endTaskLink,
} from '$lib/server/crm-task-links';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = (event) =>
  loadTaskLinks(event, event.params.workspace, event.params.record, true);
export const actions: Actions = {
  create: (event) =>
    addTaskLink(event, event.params.workspace, event.params.record, true, true),
  link: (event) =>
    addTaskLink(
      event,
      event.params.workspace,
      event.params.record,
      true,
      false,
    ),
  end: (event) =>
    endTaskLink(event, event.params.workspace, event.params.record, true),
};

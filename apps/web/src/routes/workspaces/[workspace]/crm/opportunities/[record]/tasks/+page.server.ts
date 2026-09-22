import {
  loadTaskLinks,
  addTaskLink,
  endTaskLink,
} from '$lib/server/crm-task-links';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = (event) =>
  loadTaskLinks(event, event.params.workspace, event.params.record, false);
export const actions: Actions = {
  create: (event) =>
    addTaskLink(
      event,
      event.params.workspace,
      event.params.record,
      false,
      true,
    ),
  link: (event) =>
    addTaskLink(
      event,
      event.params.workspace,
      event.params.record,
      false,
      false,
    ),
  end: (event) =>
    endTaskLink(event, event.params.workspace, event.params.record, false),
};

import { participantPage, addParticipant } from '$lib/server/crm-participants';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = (event) => participantPage(event, 'deals');
export const actions: Actions = {
  default: (event) => addParticipant(event, 'deals'),
};

import {
  participantHistory,
  editParticipant,
} from '$lib/server/crm-participants';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = (event) =>
  participantHistory(event, 'opportunities');
export const actions: Actions = {
  default: (event) => editParticipant(event, 'opportunities'),
};

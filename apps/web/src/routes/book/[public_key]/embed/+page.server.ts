import {
  loadPublicBooking,
  reservePublicBooking,
} from '$lib/server/public-booking';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = loadPublicBooking;
export const actions: Actions = { default: reservePublicBooking };

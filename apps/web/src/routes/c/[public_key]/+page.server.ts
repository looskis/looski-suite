import { loadPublicIndex } from '$lib/server/public-content';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = loadPublicIndex;

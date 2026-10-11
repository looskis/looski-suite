import { loadPublicForm, submitPublicForm } from '$lib/server/public-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = loadPublicForm;
export const actions: Actions = { default: submitPublicForm };

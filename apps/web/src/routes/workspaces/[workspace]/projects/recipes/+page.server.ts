import { fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  configurationDraft,
  configurationInput,
  recipeChoices,
} from '$lib/server/project-recipe';
import { retryKey, text } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const recipes = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/recipes',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          limit: 50,
          after: event.url.searchParams.get('after') ?? undefined,
        },
      },
    },
  );
  return {
    workspace,
    page: recipes.data ?? null,
    denied: recipes.data ? null : (recipes.error?.message ?? 'Unavailable'),
    choices: await recipeChoices(client, workspace.id),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  create: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const key = retryKey(form);
    const name = text(form, 'name');
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/recipes',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': key },
        },
        body: { name, configuration: configurationInput(form) },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create the recipe',
        name,
        draft: configurationDraft(form),
        key,
      });
    redirect(
      303,
      `/workspaces/${workspace.id}/projects/recipes/${result.data.id}`,
    );
  },
};

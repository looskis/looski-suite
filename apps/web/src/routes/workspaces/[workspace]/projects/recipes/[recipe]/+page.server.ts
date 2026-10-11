import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  configurationDraft,
  configurationInput,
  recipeChoices,
} from '$lib/server/project-recipe';
import { retryKey, text, version } from '$lib/server/project-form';
import type { Actions, PageServerLoad, RequestEvent } from './$types';
async function session(event: RequestEvent) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  return {
    client,
    form: await event.request.formData(),
    path: { workspace_id: workspace.id, recipe_id: event.params.recipe },
  };
}
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, recipe_id: event.params.recipe };
  const recipe = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}',
    { params: { path } },
  );
  if (!recipe.data)
    error(
      recipe.response.status,
      recipe.error?.message ?? 'Recipe unavailable',
    );
  const runs = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}/runs',
    {
      params: {
        path,
        query: {
          limit: 50,
          after: event.url.searchParams.get('after') ?? undefined,
        },
      },
    },
  );
  return {
    workspace,
    recipe: recipe.data,
    runs: runs.data ?? null,
    choices: await recipeChoices(client, workspace.id),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  save: async (event) => {
    const { client, path, form } = await session(event);
    const name = text(form, 'name');
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}',
      {
        params: { path, header: { 'Idempotency-Key': retryKey(form) } },
        body: {
          name,
          configuration: configurationInput(form),
          expected_version: version(form),
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save the recipe',
        name,
        draft: configurationDraft(form),
      });
    redirect(303, event.url.pathname);
  },
  active: async (event) => {
    const { client, path, form } = await session(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}/active',
      {
        params: { path, header: { 'Idempotency-Key': retryKey(form) } },
        body: {
          active: form.get('active') === 'true',
          expected_version: version(form) ?? 0,
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change the recipe',
      });
    redirect(303, event.url.pathname);
  },
  retry: async (event) => {
    const { client, path, form } = await session(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/projects/recipes/{recipe_id}/runs/{run_id}/retry',
      {
        params: {
          path: { ...path, run_id: String(form.get('run_id') ?? '') },
          header: { 'Idempotency-Key': retryKey(form) },
        },
        body: { expected_version: version(form) ?? 0 },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not retry this run',
      });
    redirect(303, event.url.pathname);
  },
};

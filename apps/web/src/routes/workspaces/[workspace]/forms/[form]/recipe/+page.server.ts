import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { mappingDraft, mappingInput } from '$lib/server/form-recipe';
import type { Actions, PageServerLoad, RequestEvent } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const workspace_id = workspace.id;
  const path = { workspace_id, form_id: event.params.form };
  const form = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}',
    { params: { path } },
  );
  if (!form.data)
    error(form.response.status, form.error?.message ?? 'Form unavailable');
  const recipe = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe',
    { params: { path } },
  );
  if (!recipe.data && recipe.response.status !== 404)
    error(
      recipe.response.status,
      recipe.error?.message ?? 'Recipe unavailable',
    );
  const versions = await client.GET(
    '/api/v1/workspaces/{workspace_id}/forms/{form_id}/versions',
    { params: { path } },
  );
  const published = versions.data?.find(
    (v) => v.id === form.data.published_version?.id,
  );
  // Run history needs submission access; the configuration does not.
  const runs = recipe.data
    ? await client.GET(
        '/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe/runs',
        {
          params: {
            path,
            query: {
              after: event.url.searchParams.get('after') ?? undefined,
              limit: 50,
            },
          },
        },
      )
    : null;
  // Choices come from CRM's own reads, which fail while CRM is off or denied.
  const pipelines = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/pipelines',
    { params: { path: { workspace_id }, query: { limit: 100 } } },
  );
  const stages: { value: string; pipeline: string; name: string }[] = [];
  for (const pipeline of pipelines.data?.items ?? []) {
    const items = await client.GET(
      '/api/v1/workspaces/{workspace_id}/crm/stages',
      {
        params: {
          path: { workspace_id },
          query: { parent_id: pipeline.id, limit: 100 },
        },
      },
    );
    for (const stage of items.data?.items ?? [])
      stages.push({
        value: `${pipeline.id}/${stage.id}`,
        pipeline: pipeline.name,
        name: stage.name,
      });
  }
  const members = await client.GET(
    '/api/v1/workspaces/{workspace_id}/access/members',
    { params: { path: { workspace_id }, query: { limit: 100 } } },
  );
  return {
    workspace,
    form: form.data,
    recipe: recipe.data ?? null,
    questions: published?.definition.fields ?? [],
    runs: runs?.data ?? null,
    crmAvailable: Boolean(pipelines.data),
    crmMessage: pipelines.data ? null : pipelines.error?.message,
    stages,
    members:
      members.data?.items
        .filter((m) => m.state === 'active')
        .map((m) => ({ id: m.id, name: m.display_name })) ?? [],
    key: randomUUID(),
  };
};
async function setup(event: RequestEvent) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const form = await event.request.formData();
  return {
    client,
    form,
    key: String(form.get('idempotency_key') ?? ''),
    path: { workspace_id: workspace.id, form_id: event.params.form },
  };
}
export const actions: Actions = {
  configure: async (event) => {
    const { client, form, key, path } = await setup(event);
    const draft = mappingDraft(form);
    const input = mappingInput(draft);
    if (!input)
      return fail(400, {
        message: 'Choose the stage where inquiries should start',
        draft,
        key,
      });
    const result = await client.PUT(
      '/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe',
      {
        params: { path, header: { 'Idempotency-Key': key } },
        body: input,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save this mapping',
        draft,
        key,
      });
    redirect(303, event.url.pathname);
  },
  activation: async (event) => {
    const { client, form, key, path } = await setup(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe/activation',
      {
        params: { path, header: { 'Idempotency-Key': key } },
        body: {
          expected_version: Number(form.get('expected_version')),
          active: form.get('active') === 'true',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change this recipe',
        key,
      });
    redirect(303, event.url.pathname);
  },
  retry: async (event) => {
    const { client, form, key, path } = await setup(event);
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/forms/{form_id}/recipe/runs/{run_id}/retry',
      {
        params: {
          path: { ...path, run_id: String(form.get('run_id') ?? '') },
          header: { 'Idempotency-Key': key },
        },
        body: { expected_version: Number(form.get('expected_version')) },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not retry this run',
        key,
      });
    redirect(303, event.url.pathname);
  },
};

import { error, fail, redirect, type RequestEvent } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { readValues } from '$lib/server/cms-form';
import { formChoices } from '$lib/server/form-choices';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
type EntryEvent = RequestEvent<{ workspace: string; entry: string }>;
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id };
  const entry = await client.GET(
    '/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}',
    { params: { path: { ...path, entry_id: event.params.entry } } },
  );
  if (!entry.data)
    error(entry.response.status, entry.error?.message ?? 'Entry unavailable');
  const [type, media, forms] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/cms/types/{type_id}', {
      params: { path: { ...path, type_id: entry.data.type_id } },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/cms/media', {
      params: { path, query: { limit: 100 } },
    }),
    formChoices(client, workspace.id),
  ]);
  if (!type.data)
    error(type.response.status, type.error?.message ?? 'Entry unavailable');
  return {
    workspace,
    entry: entry.data,
    type: type.data,
    media: media.data?.items ?? [],
    forms,
    key: randomUUID(),
  };
};
async function save(event: EntryEvent, form: FormData) {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id };
  const entry = await client.GET(
    '/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}',
    { params: { path: { ...path, entry_id: event.params.entry } } },
  );
  const type = entry.data
    ? await client.GET(
        '/api/v1/workspaces/{workspace_id}/cms/types/{type_id}',
        {
          params: { path: { ...path, type_id: entry.data.type_id } },
        },
      )
    : null;
  if (!type?.data) return null;
  return client.PATCH(
    '/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}',
    {
      params: {
        path: { ...path, entry_id: event.params.entry },
        header: {
          'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
        },
      },
      body: {
        expected_version: Number(form.get('expected_version')),
        title: String(form.get('title') ?? ''),
        slug: String(form.get('slug') ?? ''),
        values: readValues(type.data.fields, form),
      },
    },
  );
}
const version = (form: FormData) => ({
  expected_version: Number(form.get('expected_version')),
});
async function change(
  event: EntryEvent,
  action: 'publish' | 'unpublish',
  key: string,
  expected: { expected_version: number },
) {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  return client.POST(
    `/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/${action}`,
    {
      params: {
        path: { workspace_id: workspace.id, entry_id: event.params.entry },
        header: { 'Idempotency-Key': key },
      },
      body: expected,
    },
  );
}
export const actions: Actions = {
  save: async (event) => {
    requireSameOrigin(event);
    const form = await event.request.formData();
    const result = await save(event, form);
    if (!result?.data)
      return fail(result?.response.status ?? 404, {
        message: result?.error?.message ?? 'Could not save this entry',
        key: String(form.get('idempotency_key') ?? ''),
        input: Object.fromEntries(form),
      });
    redirect(303, event.url.pathname);
  },
  // Save first; if publishing then fails the draft stays saved and the page says so.
  saveAndPublish: async (event) => {
    requireSameOrigin(event);
    const form = await event.request.formData();
    const saved = await save(event, form);
    if (!saved?.data)
      return fail(saved?.response.status ?? 404, {
        message: saved?.error?.message ?? 'Could not save this entry',
        key: String(form.get('idempotency_key') ?? ''),
        input: Object.fromEntries(form),
      });
    const published = await change(
      event,
      'publish',
      String(form.get('publish_key') ?? ''),
      { expected_version: saved.data.version },
    );
    // An unchanged draft that is already live has nothing further to do.
    if (!published.data && saved.data.unpublished_changes)
      return fail(published.response.status, {
        message: `Your draft was saved but not published: ${published.error?.message ?? 'please retry'}`,
      });
    redirect(303, event.url.pathname);
  },
  unpublish: async (event) => {
    requireSameOrigin(event);
    const form = await event.request.formData();
    const result = await change(
      event,
      'unpublish',
      String(form.get('idempotency_key') ?? ''),
      version(form),
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not unpublish this entry',
      });
    redirect(303, event.url.pathname);
  },
  archive: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/cms/entries/{entry_id}/archive',
      {
        params: {
          path: { workspace_id: workspace.id, entry_id: event.params.entry },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: {
          ...version(form),
          archived: form.get('archived') === 'true',
        },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change this entry',
      });
    redirect(303, event.url.pathname);
  },
};

import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  activityFields,
  activityDraft,
  activityInput,
} from '$lib/server/activity-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, id: event.params.activity };
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/activities/{id}',
    { params: { path } },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Activity unavailable',
    );
  const history = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/activities/{id}/history',
    {
      params: {
        path,
        query: {
          limit: 50,
          after: event.url.searchParams.get('history_after') ?? undefined,
        },
      },
    },
  );
  if (!history.data)
    error(
      history.response.status,
      history.error?.message ?? 'History unavailable',
    );
  const types = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/activity-types',
    { params: { path: { workspace_id: workspace.id }, query: { limit: 100 } } },
  );
  return {
    types: types.data?.items ?? [],
    workspace,
    activity: result.data,
    fields: activityFields(result.data),
    history: history.data,
    key: randomUUID(),
    archiveKey: randomUUID(),
  };
};
export const actions: Actions = {
  save: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const fields = activityInput(form);
    const version = Number(form.get('expected_version'));
    if (!fields || !Number.isSafeInteger(version) || version < 1)
      return fail(400, {
        message: 'Valid duration and version required',
        draft: activityDraft(form),
      });
    const result = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/crm/activities/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.activity },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body: { ...fields, expected_version: version },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not save activity',
        draft: activityDraft(form),
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
    const archived = String(form.get('archived'));
    const version = Number(form.get('expected_version'));
    const archiveDraft = {
      archived,
      expected_version: String(form.get('expected_version') ?? ''),
      key: String(form.get('idempotency_key') ?? ''),
    };
    if (
      !['true', 'false'].includes(archived) ||
      !Number.isSafeInteger(version) ||
      version < 1
    )
      return fail(400, {
        message: 'Valid archive state and version required',
        archiveDraft,
      });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/crm/activities/{id}/archive',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.activity },
          header: { 'Idempotency-Key': archiveDraft.key },
        },
        body: { archived: archived === 'true', expected_version: version },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not change archive state',
        archiveDraft,
      });
    redirect(303, event.url.pathname);
  },
};

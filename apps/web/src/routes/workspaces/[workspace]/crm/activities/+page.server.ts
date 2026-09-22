import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import {
  activityFields,
  activityDraft,
  activityInput,
  anchorKind,
} from '$lib/server/activity-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const kind = anchorKind(event.url.searchParams.get('anchor_kind'));
  const id = event.url.searchParams.get('anchor_id');
  if (!kind || !id)
    error(400, 'Choose a contact, opportunity or deal to view its activities');
  const archived = event.url.searchParams.get('archived') ?? 'false';
  if (!['true', 'false'].includes(archived))
    error(400, 'Invalid archive filter');
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/activities',
    {
      params: {
        path: { workspace_id: workspace.id },
        query: {
          anchor_kind: kind,
          anchor_id: id,
          limit: 50,
          after: event.url.searchParams.get('after') ?? undefined,
          archived: archived === 'true',
          type_id: event.url.searchParams.get('type_id') || undefined,
        },
      },
    },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Activities unavailable',
    );
  const types = await client.GET(
    '/api/v1/workspaces/{workspace_id}/crm/activity-types',
    { params: { path: { workspace_id: workspace.id }, query: { limit: 100 } } },
  );
  return {
    workspace,
    typeFilter: event.url.searchParams.get('type_id') ?? '',
    page: result.data,
    types: types.data?.items ?? [],
    kind,
    id,
    archived: archived === 'true',
    fields: activityFields(),
    key: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const draft = activityDraft(form);
    const fields = activityInput(form);
    const kind = anchorKind(draft.anchor_kind);
    if (!fields || !kind)
      return fail(400, {
        message: 'Valid anchor and duration required',
        draft,
      });
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/crm/activities',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': draft.key },
        },
        body: { ...fields, anchor_kind: kind, anchor_id: draft.anchor_id },
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not record activity',
        draft,
      });
    redirect(
      303,
      `/workspaces/${workspace.id}/crm/activities/${result.data.id}`,
    );
  },
};

import { error, fail, redirect, type RequestEvent } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from './session';
type Sales = 'opportunities' | 'deals';
const paths = {
  opportunities: {
    parent: '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}',
    contacts: {
      list: '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/contacts',
      edit: '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/contacts/{association_id}',
      history:
        '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/contacts/{association_id}/history',
    },
    organizations: {
      list: '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/organizations',
      edit: '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/organizations/{association_id}',
      history:
        '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/organizations/{association_id}/history',
    },
  },
  deals: {
    parent: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}',
    contacts: {
      list: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/contacts',
      edit: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/contacts/{association_id}',
      history:
        '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/contacts/{association_id}/history',
    },
    organizations: {
      list: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/organizations',
      edit: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/organizations/{association_id}',
      history:
        '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/organizations/{association_id}/history',
    },
  },
} as const;
async function context(event: RequestEvent, sales: Sales) {
  const kind = event.params.kind;
  if (kind !== 'contacts' && kind !== 'organizations')
    error(404, 'Participant kind not found');
  const id = event.params.record;
  if (!id || !event.params.workspace) error(404, 'Record not found');
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  return {
    workspace,
    client,
    kind,
    id,
    paths: paths[sales][kind],
    parentPath: paths[sales].parent,
  };
}
export async function participantPage(event: RequestEvent, sales: Sales) {
  const { workspace, client, kind, id, paths, parentPath } = await context(
    event,
    sales,
  );
  const parent = await client.GET(parentPath, {
    params: { path: { workspace_id: workspace.id, id } },
  });
  if (!parent.data)
    error(
      parent.response.status,
      parent.error?.message ?? 'Record unavailable',
    );
  const ended = event.url.searchParams.get('ended') === 'true';
  const page = await client.GET(paths.list, {
    params: {
      path: { workspace_id: workspace.id, id },
      query: {
        limit: 50,
        after: event.url.searchParams.get('after') ?? undefined,
        ended,
      },
    },
  });
  if (!page.data)
    error(
      page.response.status,
      page.error?.message ?? 'Participants unavailable',
    );
  const search = event.url.searchParams.get('search') ?? '';
  const query = { limit: 100, search };
  const path = { workspace_id: workspace.id };
  const choices =
    kind === 'contacts'
      ? await client.GET('/api/v1/workspaces/{workspace_id}/contacts', {
          params: { path, query },
        })
      : await client.GET('/api/v1/workspaces/{workspace_id}/organizations', {
          params: { path, query },
        });
  if (!choices.data && choices.response.status !== 403)
    error(
      choices.response.status,
      choices.error?.message ?? 'References unavailable',
    );
  return {
    workspace,
    sales,
    kind,
    record: parent.data,
    page: page.data,
    ended,
    search,
    choices:
      choices.data?.items.map((r) => ({
        id: r.id,
        name: 'display_name' in r ? r.display_name : r.name,
      })) ?? [],
    moreChoices: Boolean(choices.data?.next_cursor),
    key: randomUUID(),
  };
}
export function participantDraft(form: FormData) {
  return {
    reference_id: String(form.get('reference_id') ?? ''),
    role_label: String(form.get('role_label') ?? ''),
    primary: String(form.get('primary') ?? 'keep'),
    state: String(form.get('state') ?? 'keep'),
    key: String(form.get('idempotency_key') ?? ''),
    expected_version: Number(form.get('expected_version')),
  };
}
export async function addParticipant(event: RequestEvent, sales: Sales) {
  requireSameOrigin(event);
  const { workspace, client, id, paths } = await context(event, sales);
  const draft = participantDraft(await event.request.formData());
  if (!['no', 'yes'].includes(draft.primary))
    return fail(400, {
      message: 'Choose a supported primary designation',
      draft,
    });
  const result = await client.POST(paths.list, {
    params: {
      path: { workspace_id: workspace.id, id },
      header: { 'Idempotency-Key': draft.key },
    },
    body: {
      reference_id: draft.reference_id,
      role_label: draft.role_label,
      is_primary: draft.primary === 'yes',
      expected_version: draft.expected_version,
    },
  });
  if (!result.data)
    return fail(result.response.status, {
      message: result.error?.message ?? 'Could not add participant',
      draft,
    });
  redirect(303, event.url.pathname + '/' + result.data.id);
}
export async function participantHistory(event: RequestEvent, sales: Sales) {
  const { workspace, client, kind, id, paths, parentPath } = await context(
    event,
    sales,
  );
  const association_id = event.params.association;
  if (!association_id) error(404, 'Association not found');
  const parent = await client.GET(parentPath, {
    params: { path: { workspace_id: workspace.id, id } },
  });
  if (!parent.data)
    error(
      parent.response.status,
      parent.error?.message ?? 'Record unavailable',
    );
  const result = await client.GET(paths.history, {
    params: {
      path: { workspace_id: workspace.id, id, association_id },
      query: {
        limit: 50,
        after: event.url.searchParams.get('after') ?? undefined,
      },
    },
  });
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Participant unavailable',
    );
  return {
    workspace,
    sales,
    kind,
    record: parent.data,
    history: result.data,
    participant: result.data.participant,
    key: randomUUID(),
  };
}
export async function editParticipant(event: RequestEvent, sales: Sales) {
  requireSameOrigin(event);
  const { workspace, client, id, paths } = await context(event, sales);
  const association_id = event.params.association;
  if (!association_id) error(404, 'Association not found');
  const draft = participantDraft(await event.request.formData());
  if (
    !['keep', 'no', 'yes'].includes(draft.primary) ||
    !['keep', 'end', 'restore'].includes(draft.state)
  )
    return fail(400, {
      message: 'Choose supported primary and association states',
      draft,
    });
  const result = await client.PATCH(paths.edit, {
    params: {
      path: { workspace_id: workspace.id, id, association_id },
      header: { 'Idempotency-Key': draft.key },
    },
    body: {
      expected_version: draft.expected_version,
      role_label: draft.role_label,
      is_primary:
        draft.primary === 'keep' ? undefined : draft.primary === 'yes',
      ended: draft.state === 'keep' ? undefined : draft.state === 'end',
    },
  });
  if (!result.data)
    return fail(result.response.status, {
      message: result.error?.message ?? 'Could not save participant',
      draft,
    });
  redirect(303, event.url.pathname);
}

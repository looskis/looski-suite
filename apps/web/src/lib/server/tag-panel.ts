import { error, fail, redirect, type RequestEvent } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from './session';
type Client = Awaited<ReturnType<typeof workspaceClient>>['client'];
const paths = {
  contacts: '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/tags',
  organizations:
    '/api/v1/workspaces/{workspace_id}/organizations/{organization_id}/tags',
  opportunities:
    '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tags',
  deals: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tags',
} as const;
export async function tagPanel(
  client: Client,
  workspace: string,
  id: string,
  kind: keyof typeof paths,
  search: string,
) {
  const current = await client.GET(paths[kind], {
    params: {
      path: {
        workspace_id: workspace,
        contact_id: id,
        organization_id: id,
        id,
      },
    },
  });
  if (!current.data) {
    if (current.response.status === 403) return null;
    error(
      current.response.status,
      current.error?.message ?? 'Tags unavailable',
    );
  }
  const catalog = await client.GET('/api/v1/workspaces/{workspace_id}/tags', {
    params: {
      path: { workspace_id: workspace },
      query: { search, limit: 100 },
    },
  });
  if (!catalog.data)
    error(
      catalog.response.status,
      catalog.error?.message ?? 'Tag catalog unavailable',
    );
  return {
    current: current.data,
    choices: [
      ...new Map(
        [...catalog.data.items, ...current.data].map((tag) => [tag.id, tag]),
      ).values(),
    ],
    search,
    more: !!catalog.data.next_cursor,
    key: randomUUID(),
  };
}
export async function saveTags(
  event: RequestEvent,
  workspaceId: string,
  id: string,
  kind: keyof typeof paths,
) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(event, workspaceId);
  const form = await event.request.formData();
  const body = {
    tag_ids: form.getAll('tag_id').map(String),
    expected_version: Number(form.get('expected_version')),
  };
  const header = {
    'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
  };
  const result = await client.PUT(paths[kind], {
    params: {
      path: {
        workspace_id: workspace.id,
        contact_id: id,
        organization_id: id,
        id,
      },
      header,
    },
    body,
  });
  if (!result.data)
    return fail(result.response.status, {
      message: result.error?.message ?? 'Could not save tags',
      tagDraft: {
        tagIds: body.tag_ids,
        key: header['Idempotency-Key'],
        expectedVersion: body.expected_version,
      },
    });
  redirect(303, event.url.pathname);
}

import { error, fail, redirect, type RequestEvent } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from './session';
type Client = Awaited<ReturnType<typeof workspaceClient>>['client'];
export async function tagPanel(
  client: Client,
  workspace: string,
  id: string,
  kind: 'contacts' | 'organizations',
  search: string,
) {
  const current =
    kind === 'contacts'
      ? await client.GET(
          '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/tags',
          { params: { path: { workspace_id: workspace, contact_id: id } } },
        )
      : await client.GET(
          '/api/v1/workspaces/{workspace_id}/organizations/{organization_id}/tags',
          {
            params: { path: { workspace_id: workspace, organization_id: id } },
          },
        );
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
  kind: 'contacts' | 'organizations',
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
  const result =
    kind === 'contacts'
      ? await client.PUT(
          '/api/v1/workspaces/{workspace_id}/contacts/{contact_id}/tags',
          {
            params: {
              path: { workspace_id: workspace.id, contact_id: id },
              header,
            },
            body,
          },
        )
      : await client.PUT(
          '/api/v1/workspaces/{workspace_id}/organizations/{organization_id}/tags',
          {
            params: {
              path: { workspace_id: workspace.id, organization_id: id },
              header,
            },
            body,
          },
        );
  if (!result.data)
    return fail(result.response.status, {
      message: result.error?.message ?? 'Could not save tags',
    });
  redirect(303, event.url.pathname);
}

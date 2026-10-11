import type { workspaceClient } from '$lib/server/session';

type Client = Awaited<ReturnType<typeof workspaceClient>>['client'];

/** Active collections this member reaches, for pickers. Bounded at 1000. */
export async function activeCollections(client: Client, workspaceId: string) {
  const items = [];
  let after: string | undefined;
  for (let page = 0; page < 10; page++) {
    const result = await client.GET(
      '/api/v1/workspaces/{workspace_id}/knowledge/collections',
      {
        params: {
          path: { workspace_id: workspaceId },
          query: { limit: 100, after },
        },
      },
    );
    if (!result.data) return { items, error: result };
    items.push(...result.data.items);
    if (!result.data.next_cursor) break;
    after = result.data.next_cursor;
  }
  return { items, error: null };
}

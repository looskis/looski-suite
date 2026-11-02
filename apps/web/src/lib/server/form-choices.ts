import type { workspaceClient } from '$lib/server/session';

/** Read every accessible active form for cross-application pickers. */
export async function formChoices(
  client: Awaited<ReturnType<typeof workspaceClient>>['client'],
  workspace_id: string,
) {
  const items = [];
  let after: string | undefined;
  do {
    const page = await client.GET('/api/v1/workspaces/{workspace_id}/forms', {
      params: { path: { workspace_id }, query: { limit: 100, after } },
    });
    // Discard partial choices if Forms is disabled or access is revoked.
    if (!page.data) return null;
    items.push(...page.data.items);
    after = page.data.next_cursor ?? undefined;
  } while (after);
  return items;
}

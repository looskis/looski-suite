import type { Client } from 'openapi-fetch';
import type { paths } from '$lib/api/schema';
export type RecipeDraft = {
  status_id: string;
  name_prefix: string;
  include_deal_owner: boolean;
  member_ids: string[];
};
export type RecipeChoices = {
  statuses: { id: string; label: string }[];
  members: { id: string; display_name: string }[];
};
export const configurationInput = (form: FormData): RecipeDraft => ({
  status_id: String(form.get('status_id') ?? ''),
  name_prefix: String(form.get('name_prefix') ?? ''),
  include_deal_owner: form.get('include_deal_owner') !== null,
  member_ids: form.getAll('member_ids').map(String),
});
export const configurationDraft = configurationInput;
/** Choices come from CRM's and Projects' own reads, which fail while off or denied. */
export async function recipeChoices(
  client: Client<paths>,
  workspace_id: string,
): Promise<RecipeChoices> {
  // A picker must offer every milestone, so read past the API's page limit.
  const categories = new Map<string, string>();
  let after: string | undefined;
  do {
    const page = await client.GET(
      '/api/v1/workspaces/{workspace_id}/crm/categories',
      { params: { path: { workspace_id }, query: { limit: 100, after } } },
    );
    for (const category of page.data?.items ?? [])
      categories.set(category.id, category.name);
    after = page.data?.next_cursor ?? undefined;
  } while (after);
  const statuses: RecipeChoices['statuses'] = [];
  do {
    const page = await client.GET(
      '/api/v1/workspaces/{workspace_id}/crm/statuses',
      { params: { path: { workspace_id }, query: { limit: 100, after } } },
    );
    for (const status of page.data?.items ?? [])
      if (status.parent_id && categories.has(status.parent_id))
        statuses.push({
          id: status.id,
          label: `${categories.get(status.parent_id)}: ${status.name}`,
        });
    after = page.data?.next_cursor ?? undefined;
  } while (after);
  const directory = await client.GET(
    '/api/v1/workspaces/{workspace_id}/projects/directory',
    { params: { path: { workspace_id } } },
  );
  return { statuses, members: directory.data ?? [] };
}

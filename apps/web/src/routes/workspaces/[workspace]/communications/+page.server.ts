import { error } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient } from '$lib/server/session';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id };
  const modules = await client.GET(
    '/api/v1/workspaces/{workspace_id}/modules',
    { params: { path } },
  );
  const module = modules.data?.find((m) => m.key === 'communications') ?? null;
  const archived = event.url.searchParams.get('archived') === 'true';
  const contact_id = event.url.searchParams.get('contact_id') || undefined;
  const search = event.url.searchParams.get('search') ?? '';
  const accountId = event.url.searchParams.get('account_id') ?? '';
  const base = {
    workspace,
    module,
    archived,
    contact_id,
    search,
    accountId,
    key: randomUUID(),
  };
  if (module && !module.enabled)
    return { ...base, conversations: null, accounts: [] };
  const conversations = await client.GET(
    '/api/v1/workspaces/{workspace_id}/communications/conversations',
    {
      params: {
        path,
        query: {
          newest_first: true,
          search: event.url.searchParams.get('search') || undefined,
          account_id: event.url.searchParams.get('account_id') || undefined,
          archived,
          contact_id,
          after: event.url.searchParams.get('after') || undefined,
          limit: 50,
        },
      },
    },
  );
  if (!conversations.data)
    error(
      conversations.response.status,
      conversations.error?.message ?? 'Inbox unavailable',
    );
  const accounts = await client.GET(
    '/api/v1/workspaces/{workspace_id}/communications/accounts',
    { params: { path } },
  );
  return {
    ...base,
    conversations: conversations.data,
    accounts: accounts.data ?? [],
  };
};

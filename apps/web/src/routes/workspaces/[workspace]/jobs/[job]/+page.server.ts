import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import type { PageServerLoad, Actions } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    ),
    path = { workspace_id: workspace.id, id: event.params.job };
  const r = await client.GET('/api/v1/workspaces/{workspace_id}/jobs/{id}', {
    params: { path },
  });
  if (!r.data) error(r.response.status, r.error?.message ?? 'Job unavailable');
  const h = await client.GET(
    '/api/v1/workspaces/{workspace_id}/jobs/{id}/history',
    {
      params: {
        path,
        query: {
          limit: 25,
          after: event.url.searchParams.get('after') ?? undefined,
        },
      },
    },
  );
  if (!h.data)
    error(h.response.status, h.error?.message ?? 'History unavailable');
  return { workspace, job: r.data, history: h.data, key: randomUUID() };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
        event,
        event.params.workspace,
      ),
      f = await event.request.formData(),
      t = (k: string) => String(f.get(k) ?? ''),
      draft = {
        key: t('idempotency_key'),
        expected_version: t('expected_version'),
        action: t('action'),
      },
      version = Number(draft.expected_version);
    if (
      !Number.isSafeInteger(version) ||
      version < 1 ||
      (draft.action !== 'resume' && draft.action !== 'cancel')
    )
      return fail(400, { message: 'Choose a valid action and version', draft });
    const r = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/jobs/{id}',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.job },
          header: { 'Idempotency-Key': draft.key },
        },
        body: { action: draft.action, expected_version: version },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not change job',
        draft,
      });
    redirect(303, event.url.pathname);
  },
};

import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';
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
  const module = modules.data?.find((m) => m.key === 'visitors') ?? null;
  const id = event.url.searchParams.get('visitor');
  const siteId = event.url.searchParams.get('site') || undefined;
  const search = event.url.searchParams.get('search') ?? '';
  const base = { workspace, module, key: randomUUID(), siteId, search };
  if (!module?.enabled)
    return { ...base, sites: [], page: null, detail: null, contacts: [] };
  const [sites, visitors] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/visitors/sites', {
      params: { path },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/visitors', {
      params: {
        path,
        query: {
          site_id: siteId,
          after: event.url.searchParams.get('after') || undefined,
        },
      },
    }),
  ]);
  if (!sites.data)
    error(sites.response.status, sites.error?.message ?? 'Sites unavailable');
  if (!visitors.data)
    error(
      visitors.response.status,
      visitors.error?.message ?? 'Visitors unavailable',
    );
  const detail = id
    ? await client.GET(
        '/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}',
        { params: { path: { ...path, visitor_id: id } } },
      )
    : null;
  if (id && !detail?.data)
    error(detail?.response.status ?? 404, 'Visitor unavailable');
  const contacts =
    id && search
      ? await client.GET('/api/v1/workspaces/{workspace_id}/contacts', {
          params: { path, query: { search, limit: 20 } },
        })
      : null;
  return {
    ...base,
    sites: sites.data,
    page: visitors.data,
    detail: detail?.data ?? null,
    contacts: contacts?.data?.items ?? [],
  };
};
export const actions: Actions = {
  module: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const r = await client.PATCH(
      '/api/v1/workspaces/{workspace_id}/modules/{module_key}',
      {
        params: {
          path: { workspace_id: workspace.id, module_key: 'visitors' },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          enabled: f.get('enabled') === 'true',
          expected_version: Number(f.get('expected_version')),
        },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not change module',
      });
    redirect(303, event.url.pathname);
  },
  site: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const id = String(f.get('id') || '');
    const r = await client.POST(
      '/api/v1/workspaces/{workspace_id}/visitors/sites',
      {
        params: {
          path: { workspace_id: workspace.id },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          id: id || null,
          name: String(f.get('name') || ''),
          origins: String(f.get('origins') || '')
            .split(/\s+/)
            .filter(Boolean),
          fingerprint_enabled: f.get('fingerprint') === 'on',
          similarity_enabled: f.get('similarity') === 'on',
          threshold: Number(f.get('threshold')),
          retention_days: Number(f.get('retention')),
          enabled: f.get('enabled') === 'on',
          expected_version: id ? Number(f.get('version')) : null,
        },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not save site',
      });
    redirect(303, `${event.url.pathname}?site=${r.data.id}`);
  },
  review: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const visitor_id = String(f.get('visitor'));
    const r = await client.POST(
      '/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}/matches',
      {
        params: {
          path: { workspace_id: workspace.id, visitor_id },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          candidate_id: String(f.get('candidate')),
          expected_version: Number(f.get('version')),
          state: String(f.get('state')),
        },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not review match',
      });
    redirect(303, `${event.url.pathname}?visitor=${visitor_id}`);
  },
  link: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const visitor_id = String(f.get('visitor'));
    const r = await client.PUT(
      '/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}/contact',
      {
        params: {
          path: { workspace_id: workspace.id, visitor_id },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          contact_id: String(f.get('contact') || '') || null,
          expected_version: Number(f.get('version')),
        },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not update attribution',
      });
    redirect(303, `${event.url.pathname}?visitor=${visitor_id}`);
  },
  delete: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    if (f.get('confirm') !== 'on')
      return fail(400, { message: 'Confirm deletion first.' });
    const r = await client.DELETE(
      '/api/v1/workspaces/{workspace_id}/visitors/{visitor_id}',
      {
        params: {
          path: {
            workspace_id: workspace.id,
            visitor_id: String(f.get('visitor')),
          },
          header: { 'Idempotency-Key': retryKey(f) },
        },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not delete visitor',
      });
    redirect(303, event.url.pathname);
  },
};

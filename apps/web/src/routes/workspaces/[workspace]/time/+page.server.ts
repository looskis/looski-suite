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
  const module = modules.data?.find((m) => m.key === 'time') ?? null;
  const query = {
    archived: event.url.searchParams.get('archived') === 'true',
    project_id: event.url.searchParams.get('project_id') || undefined,
    from_on: event.url.searchParams.get('from_on') || undefined,
    through_on: event.url.searchParams.get('through_on') || undefined,
  };
  const base = { workspace, module, key: randomUUID(), query };
  if (module && !module.enabled)
    return { ...base, entries: null, report: null, budgets: [], projects: [] };
  const [entries, report, budgets, projects] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/time/entries', {
      params: {
        path,
        query: {
          ...query,
          after: event.url.searchParams.get('after') || undefined,
          limit: 50,
        },
      },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/time/report', {
      params: { path, query },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/time/budgets', {
      params: { path, query: { limit: 100 } },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/projects', {
      params: { path, query: { limit: 100 } },
    }),
  ]);
  if (!entries.data)
    error(
      entries.response.status,
      entries.error?.message ?? 'Time unavailable',
    );
  if (!report.data)
    error(
      report.response.status,
      report.error?.message ?? 'Report unavailable',
    );
  const progress = await Promise.all(
    (budgets.data?.items ?? []).map(async (b) => {
      const usage = await client.GET(
        '/api/v1/workspaces/{workspace_id}/time/report',
        {
          params: {
            path,
            query: {
              from_on: b.from_on,
              through_on: b.through_on,
              project_id: b.project_id ?? undefined,
              billable: b.billable_only ? true : undefined,
            },
          },
        },
      );
      return { ...b, used: usage.data?.duration_ms ?? null };
    }),
  );
  return {
    ...base,
    entries: entries.data,
    report: report.data,
    budgets: progress,
    projects: projects.data?.items ?? [],
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
          path: { workspace_id: workspace.id, module_key: 'time' },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          enabled: f.get('enabled') === 'true',
          expected_version: Number(f.get('expected_version')),
        },
      },
    );
    if (!r.data) return fail(r.response.status, { message: r.error?.message });
    redirect(303, event.url.pathname);
  },
  save: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const id = String(f.get('id') || '');
    const body = {
      description: String(f.get('description') || ''),
      work_on: String(f.get('work_on') || ''),
      project_id: String(f.get('project_id') || '') || null,
      billable: f.get('billable') === 'on',
      duration_ms: Math.round(Number(f.get('minutes')) * 60000),
      start_timer: f.get('mode') === 'timer',
      archived: f.get('archived') === 'on',
      expected_version: id ? Number(f.get('expected_version')) : null,
    };
    const header = { 'Idempotency-Key': retryKey(f) };
    const r = id
      ? await client.PUT(
          '/api/v1/workspaces/{workspace_id}/time/entries/{id}',
          {
            params: { path: { workspace_id: workspace.id, id }, header },
            body,
          },
        )
      : await client.POST('/api/v1/workspaces/{workspace_id}/time/entries', {
          params: { path: { workspace_id: workspace.id }, header },
          body,
        });
    if (!r.data) return fail(r.response.status, { message: r.error?.message });
    redirect(303, event.url.pathname);
  },
  transition: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const r = await client.POST(
      '/api/v1/workspaces/{workspace_id}/time/entries/{id}/transition',
      {
        params: {
          path: { workspace_id: workspace.id, id: String(f.get('id')) },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          expected_version: Number(f.get('expected_version')),
          action: String(f.get('action')),
        },
      },
    );
    if (!r.data) return fail(r.response.status, { message: r.error?.message });
    redirect(303, event.url.pathname);
  },
  budget: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const id = String(f.get('id') || '');
    const body = {
      name: String(f.get('name') || ''),
      project_id: String(f.get('project_id') || '') || null,
      from_on: String(f.get('from_on') || ''),
      through_on: String(f.get('through_on') || ''),
      budget_ms: Math.round(Number(f.get('hours')) * 3600000),
      billable_only: f.get('billable_only') === 'on',
      archived: f.get('archived') === 'on',
      expected_version: id ? Number(f.get('expected_version')) : null,
    };
    const header = { 'Idempotency-Key': retryKey(f) };
    const r = id
      ? await client.PUT(
          '/api/v1/workspaces/{workspace_id}/time/budgets/{id}',
          {
            params: { path: { workspace_id: workspace.id, id }, header },
            body,
          },
        )
      : await client.POST('/api/v1/workspaces/{workspace_id}/time/budgets', {
          params: { path: { workspace_id: workspace.id }, header },
          body,
        });
    if (!r.data) return fail(r.response.status, { message: r.error?.message });
    redirect(303, event.url.pathname);
  },
};

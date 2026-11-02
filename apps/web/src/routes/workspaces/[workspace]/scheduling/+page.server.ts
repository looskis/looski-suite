import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey } from '$lib/server/project-form';
import { formChoices } from '$lib/server/form-choices';
import type { Actions, PageServerLoad } from './$types';
import type { components } from '$lib/api/schema';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    ),
    path = { workspace_id: workspace.id };
  const modules = await client.GET(
    '/api/v1/workspaces/{workspace_id}/modules',
    { params: { path } },
  );
  const module = modules.data?.find((m) => m.key === 'scheduling') ?? null;
  const archived = event.url.searchParams.get('archived') === 'true';
  const base = {
    workspace,
    module,
    key: randomUUID(),
    archived,
    origin: event.url.origin,
  };
  const empty = {
    types: null,
    appointments: null,
    editing: null,
    forms: [],
    fields: [],
    stages: [],
  };
  if (module && !module.enabled) return { ...base, ...empty };
  const [types, appointments, forms, pipelines] = await Promise.all([
    client.GET('/api/v1/workspaces/{workspace_id}/scheduling/types', {
      params: { path, query: { limit: 100, archived } },
    }),
    client.GET('/api/v1/workspaces/{workspace_id}/scheduling/appointments', {
      params: {
        path,
        query: {
          limit: 50,
          archived,
          after: event.url.searchParams.get('after') || undefined,
        },
      },
    }),
    formChoices(client, workspace.id),
    client.GET('/api/v1/workspaces/{workspace_id}/crm/pipelines', {
      params: { path, query: { limit: 100 } },
    }),
  ]);
  if (!types.data)
    error(
      types.response.status,
      types.error?.message ?? 'Scheduling unavailable',
    );
  if (!appointments.data)
    error(
      appointments.response.status,
      appointments.error?.message ?? 'Bookings unavailable',
    );
  const id = event.url.searchParams.get('type');
  const editing = id
    ? await client.GET(
        '/api/v1/workspaces/{workspace_id}/scheduling/types/{id}',
        { params: { path: { ...path, id } } },
      )
    : null;
  if (id && !editing?.data)
    error(editing?.response.status ?? 404, 'Appointment type unavailable');
  const form_id = editing?.data?.settings.form_id;
  const form = form_id
    ? await client.GET('/api/v1/workspaces/{workspace_id}/forms/{form_id}', {
        params: { path: { ...path, form_id } },
      })
    : null;
  const versions = form_id
    ? await client.GET(
        '/api/v1/workspaces/{workspace_id}/forms/{form_id}/versions',
        { params: { path: { ...path, form_id } } },
      )
    : null;
  const fields =
    versions?.data?.find((v) => v.id === form?.data?.published_version?.id)
      ?.definition.fields ?? [];
  const stages: { value: string; name: string }[] = [];
  for (const pipeline of pipelines.data?.items ?? []) {
    const stagesResult = await client.GET(
      '/api/v1/workspaces/{workspace_id}/crm/stages',
      { params: { path, query: { parent_id: pipeline.id, limit: 100 } } },
    );
    for (const s of stagesResult.data?.items ?? [])
      stages.push({
        value: `${pipeline.id}/${s.id}`,
        name: `${pipeline.name} · ${s.name}`,
      });
  }
  return {
    ...base,
    types: types.data,
    appointments: appointments.data,
    editing: editing?.data ?? null,
    forms: forms ?? [],
    fields,
    stages,
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
          path: { workspace_id: workspace.id, module_key: 'scheduling' },
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
    const text = (k: string) => String(f.get(k) ?? '');
    const minute = (k: string) => {
      const [h, m] = text(k).split(':').map(Number);
      return h * 60 + m;
    };
    const weekly = [];
    for (let i = 0; i < Math.min(50, Number(f.get('weekly_count'))); i++)
      weekly.push({
        weekday: Number(f.get(`day_${i}`)),
        start: minute(`start_${i}`),
        end: minute(`end_${i}`),
      });
    const exceptions: components['schemas']['AvailabilityException'][] = [];
    for (let i = 0; i < Math.min(100, Number(f.get('exception_count'))); i++) {
      const date = text(`date_${i}`);
      let x = exceptions.find((x) => x.date === date);
      if (!x) {
        x = { date, windows: [] };
        exceptions.push(x);
      }
      if (f.get(`closed_${i}`) !== 'on')
        x.windows.push({
          start: minute(`exception_start_${i}`),
          end: minute(`exception_end_${i}`),
        });
    }
    const [pipeline_id, stage_id] = text('stage').split('/');
    const settings: components['schemas']['AppointmentSettings'] = {
      name: text('name'),
      description: text('description'),
      timezone: text('timezone'),
      duration_minutes: Number(f.get('duration_minutes')),
      buffer_before: Number(f.get('buffer_before')),
      buffer_after: Number(f.get('buffer_after')),
      reminder_minutes: Number(f.get('reminder_minutes')),
      accepting: f.get('accepting') === 'on',
      archived: f.get('archived') === 'on',
      weekly,
      exceptions,
      form_id: text('form_id') || null,
      crm:
        f.get('crm') === 'on'
          ? {
              name_fields: f.getAll('name_fields').map(String),
              email_field: text('email_field') || null,
              phone_field: text('phone_field') || null,
              phone_country: text('phone_country') || null,
              pipeline_id,
              stage_id,
              title_prefix: text('title_prefix'),
              owner_membership_id: text('owner_membership_id') || null,
            }
          : null,
    };
    const id = text('id'),
      header = { 'Idempotency-Key': retryKey(f) },
      body = {
        settings,
        expected_version: id ? Number(f.get('expected_version')) : null,
      };
    const r = id
      ? await client.PUT(
          '/api/v1/workspaces/{workspace_id}/scheduling/types/{id}',
          {
            params: { path: { workspace_id: workspace.id, id }, header },
            body,
          },
        )
      : await client.POST(
          '/api/v1/workspaces/{workspace_id}/scheduling/types',
          { params: { path: { workspace_id: workspace.id }, header }, body },
        );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not save appointment type',
        draft: settings,
      });
    redirect(303, `${event.url.pathname}?type=${r.data.id}`);
  },
};

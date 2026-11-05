import { error, fail, redirect, type RequestEvent } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from './session';
import { defaultColumns, type ViewConfig } from '$lib/records';
import type { components } from '$lib/api/schema';
type Client = Awaited<ReturnType<typeof workspaceClient>>['client'];
export async function recordFields(
  client: Client,
  workspace: string,
  kind: string,
) {
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/records/{kind}/fields',
    { params: { path: { workspace_id: workspace, kind } } },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Fields unavailable',
    );
  return result.data;
}
export async function recordView(
  client: Client,
  workspace: string,
  kind: string,
  url: URL,
) {
  const [fields, views] = await Promise.all([
    recordFields(client, workspace, kind),
    client.GET('/api/v1/workspaces/{workspace_id}/records/{kind}/views', {
      params: { path: { workspace_id: workspace, kind } },
    }),
  ]);
  if (!views.data)
    error(views.response.status, views.error?.message ?? 'Views unavailable');
  const selected = views.data.find(
    (v) => v.id === url.searchParams.get('view'),
  );
  const config: ViewConfig = selected?.config ?? {
    columns: defaultColumns(kind),
    filter: null,
    sort: 'name',
    direction: 'asc',
    archived: false,
  };
  if (url.searchParams.has('columns'))
    config.columns = url.searchParams.get('columns')!.split(',');
  config.columns = config.columns
    .filter(
      (key, i, list) =>
        fields.some((f) => f.key === key) && list.indexOf(key) === i,
    )
    .slice(0, 40);
  if (!config.columns.length) config.columns = defaultColumns(kind);
  if (url.searchParams.has('filter')) {
    try {
      config.filter = JSON.parse(url.searchParams.get('filter')!);
    } catch {
      error(400, 'Invalid filter expression');
    }
  }
  config.sort = url.searchParams.get('sort') ?? config.sort;
  config.direction = url.searchParams.get('direction') ?? config.direction;
  if (url.searchParams.has('archived'))
    config.archived = url.searchParams.get('archived') === 'true';
  return {
    fields,
    views: views.data,
    viewId: selected?.id ?? '',
    config,
    viewKey: randomUUID(),
  };
}
export async function saveView(event: RequestEvent, kind: string) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace!,
  );
  const form = await event.request.formData();
  let config: components['schemas']['ViewConfig'];
  try {
    config = JSON.parse(String(form.get('config')));
  } catch {
    return fail(400, { message: 'Invalid view configuration' });
  }
  const body = {
    name: String(form.get('name') ?? ''),
    shared: form.get('shared') === 'on',
    config,
    expected_version: form.get('version') ? Number(form.get('version')) : null,
  };
  const id = String(form.get('view_id') ?? '');
  const header = { 'Idempotency-Key': String(form.get('view_key') ?? '') };
  const result = id
    ? await client.PUT(
        '/api/v1/workspaces/{workspace_id}/records/{kind}/views/{view_id}',
        {
          params: {
            path: { workspace_id: workspace.id, kind, view_id: id },
            header,
          },
          body,
        },
      )
    : await client.POST(
        '/api/v1/workspaces/{workspace_id}/records/{kind}/views',
        {
          params: { path: { workspace_id: workspace.id, kind }, header },
          body,
        },
      );
  if (!result.data)
    return fail(result.response.status, {
      message: result.error?.message ?? 'Could not save view',
    });
  redirect(303, `${event.url.pathname}?view=${result.data.id}`);
}
export function recordProperties(
  form: FormData,
): { [key: string]: unknown } | undefined {
  if (!form.has('properties_present')) return undefined;
  let properties: { [key: string]: unknown } = {};
  try {
    properties = JSON.parse(String(form.get('properties_original') ?? '{}'));
  } catch {
    error(400, 'Invalid record properties');
  }
  const location: { [key: string]: string } = {};
  for (const [name, value] of form.entries()) {
    if (!name.startsWith('property:')) continue;
    const [, type, ...parts] = name.split(':');
    const key = parts.join(':');
    const text = String(value).trim();
    if (key.startsWith('location.')) {
      if (text) location[key.slice(9)] = text;
      continue;
    }
    properties[key] =
      text === ''
        ? null
        : type === 'number'
          ? Number(text)
          : type === 'boolean'
            ? text === 'true'
            : text;
  }
  properties.location = location;
  return properties;
}
export async function deleteView(event: RequestEvent, kind: string) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace!,
  );
  const form = await event.request.formData();
  const result = await client.DELETE(
    '/api/v1/workspaces/{workspace_id}/records/{kind}/views/{view_id}',
    {
      params: {
        path: {
          workspace_id: workspace.id,
          kind,
          view_id: String(form.get('view_id')),
        },
        header: { 'Idempotency-Key': String(form.get('view_key')) },
      },
      body: { expected_version: Number(form.get('version')) },
    },
  );
  if (!result.data)
    return fail(result.response.status, {
      message: result.error?.message ?? 'Could not delete view',
    });
  redirect(303, event.url.pathname);
}

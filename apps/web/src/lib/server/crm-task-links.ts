import type { TaskLinksData } from '$lib/crm-task-links';
import { error, fail, redirect } from '@sveltejs/kit';
import type { RequestEvent } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { taskFields, taskDraft, taskInput } from '$lib/server/task-form';
function paths(deal: boolean) {
  return deal
    ? ({
        record: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}',
        list: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tasks',
        create: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/followups',
        edit: '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tasks/{link_id}',
        history:
          '/api/v1/workspaces/{workspace_id}/crm/deals/{id}/tasks/{link_id}/history',
      } as const)
    : ({
        record: '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}',
        list: '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tasks',
        create:
          '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/followups',
        edit: '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tasks/{link_id}',
        history:
          '/api/v1/workspaces/{workspace_id}/crm/opportunities/{id}/tasks/{link_id}/history',
      } as const);
}
function draft(form: FormData, mode: string) {
  return {
    ...taskDraft(form),
    task_id: String(form.get('task_id') ?? ''),
    mode,
  };
}
function endDraft(form: FormData) {
  return {
    link_id: String(form.get('link_id') ?? ''),
    ended: String(form.get('ended') ?? ''),
    expected_version: String(form.get('expected_version') ?? ''),
    key: String(form.get('idempotency_key') ?? ''),
  };
}
export async function loadTaskLinks(
  event: RequestEvent,
  workspaceId: string,
  id: string,
  deal: boolean,
): Promise<TaskLinksData> {
  const { workspace, client } = await workspaceClient(event, workspaceId);
  const endpoints = paths(deal);
  const path = { workspace_id: workspace.id, id };
  const record = await client.GET(endpoints.record, { params: { path } });
  if (!record.data)
    error(
      record.response.status,
      record.error?.message ?? 'Record unavailable',
    );
  const ended = event.url.searchParams.get('ended') === 'true';
  const search = event.url.searchParams.get('task_search') ?? '';
  const links = await client.GET(endpoints.list, {
    params: {
      path,
      query: {
        limit: 50,
        after: event.url.searchParams.get('after') ?? undefined,
        ended,
      },
    },
  });
  if (!links.data)
    error(
      links.response.status,
      links.error?.message ?? 'Task links unavailable',
    );
  const choices = await client.GET('/api/v1/workspaces/{workspace_id}/tasks', {
    params: {
      path: { workspace_id: workspace.id },
      query: { limit: 100, search },
    },
  });
  if (!choices.data && choices.response.status !== 403)
    error(
      choices.response.status,
      choices.error?.message ?? 'Tasks unavailable',
    );
  return {
    workspace,
    record: record.data,
    links: links.data,
    choices: choices.data?.items ?? [],
    search,
    ended,
    sales: deal ? 'deals' : 'opportunities',
    fields: taskFields(),
    createKey: randomUUID(),
    linkKey: randomUUID(),
    endKeys: Object.fromEntries(
      links.data.items.map((link) => [link.id, randomUUID()]),
    ),
  };
}
export async function addTaskLink(
  event: RequestEvent,
  workspaceId: string,
  id: string,
  deal: boolean,
  creating: boolean,
) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(event, workspaceId);
  const endpoints = paths(deal);
  const form = await event.request.formData();
  const version = Number(form.get('expected_version'));
  const mode = creating ? 'create' : 'link';
  if (!Number.isSafeInteger(version) || version < 1)
    return fail(400, {
      message: 'A valid record version is required',
      draft: draft(form, mode),
    });
  const params = {
    path: { workspace_id: workspace.id, id },
    header: { 'Idempotency-Key': String(form.get('idempotency_key') ?? '') },
  };
  if (creating) {
    const task = taskInput(form);
    if (!task)
      return fail(400, {
        message: 'Unsupported task status',
        draft: draft(form, mode),
      });
    const result = await client.POST(endpoints.create, {
      params,
      body: { expected_version: version, task },
    });
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not create follow-up',
        draft: draft(form, mode),
      });
  } else {
    const result = await client.POST(endpoints.list, {
      params,
      body: {
        expected_version: version,
        task_id: String(form.get('task_id') ?? ''),
      },
    });
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not link task',
        draft: draft(form, mode),
      });
  }
  redirect(303, event.url.pathname);
}
export async function endTaskLink(
  event: RequestEvent,
  workspaceId: string,
  id: string,
  deal: boolean,
) {
  requireSameOrigin(event);
  const { workspace, client } = await workspaceClient(event, workspaceId);
  const form = await event.request.formData();
  const values = endDraft(form);
  const version = Number(values.expected_version);
  if (
    !['true', 'false'].includes(values.ended) ||
    !Number.isSafeInteger(version) ||
    version < 1
  )
    return fail(400, {
      message: 'Valid link state and record version required',
      endDraft: values,
    });
  const result = await client.PATCH(paths(deal).edit, {
    params: {
      path: { workspace_id: workspace.id, id, link_id: values.link_id },
      header: { 'Idempotency-Key': values.key },
    },
    body: { expected_version: version, ended: values.ended === 'true' },
  });
  if (!result.data)
    return fail(result.response.status, {
      message: result.error?.message ?? 'Could not change link state',
      endDraft: values,
    });
  redirect(
    303,
    event.url.pathname + '?ended=' + String(values.ended === 'true'),
  );
}
export async function loadTaskLinkHistory(
  event: RequestEvent,
  workspaceId: string,
  id: string,
  linkId: string,
  deal: boolean,
) {
  const { workspace, client } = await workspaceClient(event, workspaceId);
  const result = await client.GET(paths(deal).history, {
    params: {
      path: { workspace_id: workspace.id, id, link_id: linkId },
      query: {
        limit: 50,
        after: event.url.searchParams.get('after') ?? undefined,
      },
    },
  });
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Task link history unavailable',
    );
  return {
    workspace,
    id,
    sales: deal ? 'deals' : 'opportunities',
    history: result.data,
  };
}

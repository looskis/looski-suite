import { error } from '@sveltejs/kit';
import type { components } from '$lib/api/schema';
import { workspaceClient } from '$lib/server/session';
type Client = Awaited<ReturnType<typeof workspaceClient>>['client'];
type Record =
  components['schemas']['Opportunity'] | components['schemas']['Deal'];
type Choice = { id: string; name: string };
export async function workflowChoices(
  client: Client,
  workspace: string,
  isDeal: boolean,
  parent: string | null,
  current?: Record,
  stateAfter?: string | null,
) {
  const parentPath = isDeal
    ? '/api/v1/workspaces/{workspace_id}/crm/categories'
    : '/api/v1/workspaces/{workspace_id}/crm/pipelines';
  const parentItem = isDeal
    ? '/api/v1/workspaces/{workspace_id}/crm/categories/{id}'
    : '/api/v1/workspaces/{workspace_id}/crm/pipelines/{id}';
  const statePath = isDeal
    ? '/api/v1/workspaces/{workspace_id}/crm/statuses'
    : '/api/v1/workspaces/{workspace_id}/crm/stages';
  const parents = await client.GET(parentPath, {
    params: { path: { workspace_id: workspace }, query: { limit: 100 } },
  });
  if (!parents.data && parents.response.status !== 403)
    error(
      parents.response.status,
      parents.error?.message ?? 'Workflow choices unavailable',
    );
  const choices: Choice[] =
    parents.data?.items.map((p) => ({ id: p.id, name: p.name })) ?? [];
  const currentParent = current
    ? 'pipeline_id' in current
      ? current.pipeline_id
      : current.category_id
    : null;
  const currentState = current
    ? 'stage_id' in current
      ? current.stage_id
      : current.status_id
    : null;
  if (current && currentParent && !choices.some((c) => c.id === currentParent))
    choices.push({
      id: currentParent,
      name:
        'pipeline_name' in current
          ? current.pipeline_name
          : current.category_name,
    });
  const selected = parent || currentParent || choices[0]?.id || '';
  if (selected && !choices.some((c) => c.id === selected)) {
    const item = await client.GET(parentItem, {
      params: { path: { workspace_id: workspace, id: selected } },
    });
    if (!item.data)
      error(
        item.response.status,
        item.error?.message ?? 'Workflow unavailable',
      );
    choices.push(item.data);
  }
  const states = selected
    ? await client.GET(statePath, {
        params: {
          path: { workspace_id: workspace },
          query: {
            parent_id: selected,
            limit: 100,
            after: stateAfter ?? undefined,
          },
        },
      })
    : null;
  if (states && !states.data && states.response.status !== 403)
    error(
      states.response.status,
      states.error?.message ?? 'States unavailable',
    );
  const stateChoices: Choice[] =
    states?.data?.items.map((s) => ({ id: s.id, name: s.name })) ?? [];
  if (
    current &&
    currentParent === selected &&
    currentState &&
    !stateChoices.some((s) => s.id === currentState)
  )
    stateChoices.push({
      id: currentState,
      name: 'stage_name' in current ? current.stage_name : current.status_name,
    });
  const members = await client.GET(
    '/api/v1/workspaces/{workspace_id}/access/members',
    { params: { path: { workspace_id: workspace }, query: { limit: 100 } } },
  );
  if (!members.data && members.response.status !== 403)
    error(
      members.response.status,
      members.error?.message ?? 'Assignees unavailable',
    );
  return {
    parents: choices,
    parent: selected,
    states: stateChoices,
    state: stateChoices.some((s) => s.id === currentState)
      ? currentState
      : (stateChoices[0]?.id ?? ''),
    members:
      members.data?.items
        .filter((m) => m.state === 'active')
        .map((m) => ({ id: m.id, name: m.display_name })) ?? [],
    moreParents: Boolean(parents.data?.next_cursor),
    moreStates: Boolean(states?.data?.next_cursor),
    nextState: states?.data?.next_cursor ?? null,
  };
}
export function recordFields(record?: Record) {
  return {
    title: record?.title ?? '',
    notes: record?.notes ?? '',
    amount: record
      ? (('pipeline_id' in record ? record.expected_value : record.value) ?? '')
      : '',
    currency: record?.currency_code ?? '',
    date1: record
      ? (('pipeline_id' in record
          ? record.expected_close_on
          : record.starts_on) ?? '')
      : '',
    date2: record && 'ends_on' in record ? (record.ends_on ?? '') : '',
    owner: record?.owner_membership_id ?? '',
  };
}
function text(form: FormData, key: string) {
  return String(form.get(key) ?? '');
}
export function opportunityInput(
  form: FormData,
): components['schemas']['OpportunityInput'] {
  return {
    title: text(form, 'title'),
    notes: text(form, 'notes'),
    owner_membership_id: text(form, 'owner') || null,
    pipeline_id: text(form, 'parent_id'),
    stage_id: text(form, 'state_id'),
    expected_value: text(form, 'amount') || null,
    currency_code: text(form, 'currency') || null,
    expected_close_on: text(form, 'date1') || null,
  };
}
export function dealInput(form: FormData): components['schemas']['DealInput'] {
  return {
    title: text(form, 'title'),
    notes: text(form, 'notes'),
    owner_membership_id: text(form, 'owner') || null,
    category_id: text(form, 'parent_id'),
    status_id: text(form, 'state_id'),
    value: text(form, 'amount') || null,
    currency_code: text(form, 'currency') || null,
    starts_on: text(form, 'date1') || null,
    ends_on: text(form, 'date2') || null,
  };
}
export function recordDraft(form: FormData) {
  return {
    title: text(form, 'title'),
    notes: text(form, 'notes'),
    amount: text(form, 'amount'),
    currency: text(form, 'currency'),
    date1: text(form, 'date1'),
    date2: text(form, 'date2'),
    owner: text(form, 'owner'),
    parent_id: text(form, 'parent_id'),
    state_id: text(form, 'state_id'),
    origin: text(form, 'origin'),
    key: text(form, 'idempotency_key'),
    expected_version: text(form, 'expected_version'),
  };
}

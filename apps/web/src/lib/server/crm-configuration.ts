import type { components } from '$lib/api/schema';
import { error } from '@sveltejs/kit';
export function configurationPaths(kind: string) {
  switch (kind) {
    case 'pipelines':
      return {
        list: '/api/v1/workspaces/{workspace_id}/crm/pipelines',
        item: '/api/v1/workspaces/{workspace_id}/crm/pipelines/{id}',
        title: 'Opportunity pipelines',
        parent: null,
      } as const;
    case 'stages':
      return {
        list: '/api/v1/workspaces/{workspace_id}/crm/stages',
        item: '/api/v1/workspaces/{workspace_id}/crm/stages/{id}',
        title: 'Opportunity stages',
        parent: '/api/v1/workspaces/{workspace_id}/crm/pipelines',
      } as const;
    case 'categories':
      return {
        list: '/api/v1/workspaces/{workspace_id}/crm/categories',
        item: '/api/v1/workspaces/{workspace_id}/crm/categories/{id}',
        title: 'Deal categories',
        parent: null,
      } as const;
    case 'statuses':
      return {
        list: '/api/v1/workspaces/{workspace_id}/crm/statuses',
        item: '/api/v1/workspaces/{workspace_id}/crm/statuses/{id}',
        title: 'Deal statuses',
        parent: '/api/v1/workspaces/{workspace_id}/crm/categories',
      } as const;
    default:
      error(404, 'Unknown CRM configuration');
  }
}
export function definitionInput(
  form: FormData,
): components['schemas']['SaveDefinition'] {
  const position = Number(form.get('position') ?? 0);
  if (
    !Number.isInteger(position) ||
    position < -2147483648 ||
    position > 2147483647
  )
    error(400, 'Position must be an integer');
  const outcome = String(form.get('outcome') ?? '');
  if (outcome && outcome !== 'open' && outcome !== 'won' && outcome !== 'lost')
    error(400, 'Choose a supported outcome');
  return {
    name: String(form.get('name') ?? ''),
    parent_id: String(form.get('parent_id') ?? '') || null,
    outcome:
      outcome === 'open' || outcome === 'won' || outcome === 'lost'
        ? outcome
        : null,
    position,
    archived: form.has('archived'),
    expected_version: form.has('expected_version')
      ? Number(form.get('expected_version'))
      : null,
  };
}

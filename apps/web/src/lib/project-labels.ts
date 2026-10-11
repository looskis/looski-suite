import type { components } from '$lib/api/schema';
type Run = components['schemas']['DealConversion'];

export const projectStatuses = [
  { value: 'active', label: 'Active' },
  { value: 'on_hold', label: 'On hold' },
  { value: 'completed', label: 'Completed' },
  { value: 'cancelled', label: 'Cancelled' },
] as const;
export const taskStatuses = [
  { value: 'open', label: 'Open' },
  { value: 'in_progress', label: 'In progress' },
  { value: 'completed', label: 'Completed' },
  { value: 'cancelled', label: 'Cancelled' },
] as const;
export const projectStatusLabel = (value: string) =>
  projectStatuses.find((s) => s.value === value)?.label ?? value;
export const taskStatusLabel = (value: string) =>
  taskStatuses.find((s) => s.value === value)?.label ?? value;

/** What a conversion's state means to the person deciding whether to retry it. */
export function runStatus(run: Run): {
  label: string;
  detail: string;
  attention: boolean;
  retriable: boolean;
} {
  if (run.job_state === 'blocked' || run.job_state === 'exhausted') {
    const reasons: Record<string, string> = {
      authority:
        'The member who activated this recipe no longer has a permission it needs. Restore it or reactivate the recipe, then retry.',
      module_disabled:
        'CRM or Projects was turned off when this ran. Turn it on, then retry.',
      invalid_input:
        'The configuration no longer fits, for example because a listed member left the workspace.',
      unavailable:
        'The deal this run needs is unavailable; it may have been archived.',
      database: 'The run failed repeatedly on a storage error. Retry it.',
      lease_expired: 'The worker stopped repeatedly during this run. Retry it.',
    };
    return {
      label: 'Failed',
      detail:
        reasons[run.error_code ?? ''] ??
        `This run stopped (${run.error_code ?? run.job_state}). Retry it.`,
      attention: true,
      retriable: true,
    };
  }
  if (run.job_state === 'cancelled')
    return {
      label: 'Cancelled',
      detail:
        'Its background work was cancelled. This deal will not convert again under this recipe.',
      attention: false,
      retriable: false,
    };
  if (run.state === 'completed')
    return {
      label: 'Project created',
      detail:
        'This deal has its project; reaching the status again changes nothing.',
      attention: false,
      retriable: false,
    };
  return {
    label: 'Queued',
    detail: 'Waiting for the background worker.',
    attention: false,
    retriable: false,
  };
}

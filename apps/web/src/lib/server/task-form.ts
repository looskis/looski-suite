import type { components } from '$lib/api/schema';
type Task = components['schemas']['Task'];
export function taskFields(task?: Task) {
  return {
    title: task?.title ?? '',
    notes: task?.notes ?? '',
    assigned_membership_id: task?.assigned_membership_id ?? '',
    due_on: task?.due_on ?? '',
    status: task?.status ?? 'open',
  };
}
export function taskDraft(form: FormData) {
  const text = (key: string) => String(form.get(key) ?? '');
  return {
    title: text('title'),
    notes: text('notes'),
    assigned_membership_id: text('assigned_membership_id'),
    due_on: text('due_on'),
    status: text('status'),
    key: text('idempotency_key'),
    expected_version: text('expected_version'),
  };
}
export function taskInput(
  form: FormData,
): components['schemas']['TaskInput'] | null {
  const draft = taskDraft(form);
  const status = draft.status;
  if (
    status !== 'open' &&
    status !== 'in_progress' &&
    status !== 'completed' &&
    status !== 'cancelled'
  )
    return null;
  return {
    title: draft.title,
    notes: draft.notes,
    assigned_membership_id: draft.assigned_membership_id || null,
    due_on: draft.due_on || null,
    status,
  };
}

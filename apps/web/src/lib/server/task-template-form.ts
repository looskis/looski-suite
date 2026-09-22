import type { components } from '$lib/api/schema';
export function templateFields(
  template?: components['schemas']['TaskTemplate'],
) {
  const r = template?.revision;
  return {
    name: template?.name ?? '',
    archived: String(template?.archived ?? false),
    title: r?.title ?? '',
    notes: r?.notes ?? '',
    type_id: r?.type_id ?? '',
    assignment: r?.assignment ?? 'initiator',
    fixed_membership_id: r?.fixed_membership_id ?? '',
    due_offset_days: String(r?.due_offset_days ?? 0),
  };
}
export function templateDraft(form: FormData) {
  const t = (k: string) => String(form.get(k) ?? '');
  return {
    name: t('name'),
    archived: t('archived'),
    title: t('title'),
    notes: t('notes'),
    type_id: t('type_id'),
    assignment: t('assignment'),
    fixed_membership_id: t('fixed_membership_id'),
    due_offset_days: t('due_offset_days'),
    expected_version: t('expected_version'),
    key: t('idempotency_key'),
  };
}
export function templateInput(
  form: FormData,
  editing: boolean,
): components['schemas']['SaveTaskTemplate'] | null {
  const d = templateDraft(form);
  const offset = Number(d.due_offset_days);
  const version = Number(d.expected_version);
  if (
    d.assignment !== 'initiator' &&
    d.assignment !== 'context_owner' &&
    d.assignment !== 'fixed'
  )
    return null;
  if (
    !Number.isSafeInteger(offset) ||
    Math.abs(offset) > 3650 ||
    !['true', 'false'].includes(d.archived) ||
    (editing && (!Number.isSafeInteger(version) || version < 1))
  )
    return null;
  return {
    name: d.name,
    archived: d.archived === 'true',
    title: d.title,
    notes: d.notes,
    type_id: d.type_id || null,
    assignment: d.assignment,
    fixed_membership_id: d.fixed_membership_id || null,
    due_offset_days: offset,
    expected_version: editing ? version : null,
  };
}

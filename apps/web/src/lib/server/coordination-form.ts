import type { components } from '$lib/api/schema';
export function coordinationFields(
  template?: components['schemas']['CoordinationTemplate'],
) {
  return {
    name: template?.name ?? '',
    archived: String(template?.archived ?? false),
    items: template?.revision.items.map((i) => ({
      label: i.label,
      task_template_revision_id: i.task_template_revision_id,
    })) ?? [{ label: '', task_template_revision_id: '' }],
  };
}
export function coordinationDraft(form: FormData) {
  const t = (k: string) => String(form.get(k) ?? '');
  const labels = form.getAll('item_label').map(String);
  const revisions = form.getAll('task_template_revision_id').map(String);
  return {
    name: t('name'),
    archived: t('archived'),
    expected_version: t('expected_version'),
    key: t('idempotency_key'),
    items: labels.map((label, index) => ({
      label,
      task_template_revision_id: revisions[index] ?? '',
    })),
  };
}
export function coordinationInput(
  form: FormData,
  editing: boolean,
): components['schemas']['SaveCoordinationTemplate'] | null {
  const d = coordinationDraft(form);
  const version = Number(d.expected_version);
  if (
    d.items.length < 1 ||
    d.items.length > 25 ||
    form.getAll('item_label').length !==
      form.getAll('task_template_revision_id').length ||
    !['true', 'false'].includes(d.archived) ||
    (editing && (!Number.isSafeInteger(version) || version < 1))
  )
    return null;
  return {
    name: d.name,
    archived: d.archived === 'true',
    expected_version: editing ? version : null,
    items: d.items,
  };
}

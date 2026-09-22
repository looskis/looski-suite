import type { components } from '$lib/api/schema';
export function typeDraft(form: FormData) {
  const t = (k: string) => String(form.get(k) ?? '');
  return {
    name: t('name'),
    behavior: t('behavior'),
    position: t('position'),
    archived: t('archived'),
    expected_version: t('expected_version'),
    key: t('idempotency_key'),
  };
}
export function typeInput(
  form: FormData,
  editing: boolean,
): components['schemas']['SaveActivityType'] | null {
  const d = typeDraft(form);
  const position = Number(d.position);
  const version = Number(d.expected_version);
  if (d.behavior !== 'note' && d.behavior !== 'interaction') return null;
  if (
    !Number.isSafeInteger(position) ||
    position < -2147483648 ||
    position > 2147483647 ||
    !['true', 'false'].includes(d.archived) ||
    (editing && (!Number.isSafeInteger(version) || version < 1))
  )
    return null;
  return {
    name: d.name,
    behavior: d.behavior,
    position,
    archived: d.archived === 'true',
    expected_version: editing ? version : null,
  };
}

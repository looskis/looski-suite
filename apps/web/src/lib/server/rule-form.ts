import type { components } from '$lib/api/schema';
type Rule = components['schemas']['CoordinationRule'];
export function ruleFields(r?: Rule) {
  return {
    name: r?.name ?? '',
    status_id: r?.status_id ?? '',
    revision_id: r?.revision_id ?? '',
    archived: String(r?.archived ?? false),
  };
}
export function ruleDraft(f: FormData) {
  const t = (k: string) => String(f.get(k) ?? '');
  return {
    name: t('name'),
    status_id: t('status_id'),
    revision_id: t('revision_id'),
    archived: t('archived'),
    key: t('idempotency_key'),
    expected_version: t('expected_version'),
  };
}
export function ruleInput(f: FormData, edit = false) {
  const d = ruleDraft(f),
    v = Number(d.expected_version);
  if (
    !['true', 'false'].includes(d.archived) ||
    (edit && (!Number.isSafeInteger(v) || v < 1))
  )
    return null;
  return {
    name: d.name,
    status_id: d.status_id,
    revision_id: d.revision_id,
    archived: d.archived === 'true',
    ...(edit ? { expected_version: v } : {}),
  };
}

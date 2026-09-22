import type { components } from '$lib/api/schema';
export function activityFields(activity?: components['schemas']['Activity']) {
  return {
    type_id: activity?.type_id ?? '',
    subject: activity?.subject ?? '',
    body: activity?.body ?? '',
    occurred_at: activity?.occurred_at ?? new Date().toISOString(),
    duration_minutes: String(activity?.duration_minutes ?? ''),
    responsible_membership_id: activity?.responsible_membership_id ?? '',
  };
}
export function activityDraft(form: FormData) {
  const t = (key: string) => String(form.get(key) ?? '');
  return {
    type_id: t('type_id'),
    subject: t('subject'),
    body: t('body'),
    occurred_at: t('occurred_at'),
    duration_minutes: t('duration_minutes'),
    responsible_membership_id: t('responsible_membership_id'),
    anchor_kind: t('anchor_kind'),
    anchor_id: t('anchor_id'),
    expected_version: t('expected_version'),
    key: t('idempotency_key'),
  };
}
export function activityInput(
  form: FormData,
): components['schemas']['ActivityInput'] | null {
  const d = activityDraft(form);
  const duration =
    d.duration_minutes === '' ? null : Number(d.duration_minutes);
  if (
    duration !== null &&
    (!Number.isSafeInteger(duration) || duration < 0 || duration > 10080)
  )
    return null;
  return {
    type_id: d.type_id,
    subject: d.subject,
    body: d.body,
    occurred_at: d.occurred_at,
    duration_minutes: duration,
    responsible_membership_id: d.responsible_membership_id || null,
  };
}
export function anchorKind(
  kind: string | null,
): components['schemas']['AnchorKind'] | null {
  return kind === 'contact' || kind === 'opportunity' || kind === 'deal'
    ? kind
    : null;
}

/** A mutation's retry key: one per rendered form, reused when it is resubmitted. */
export const retryKey = (form: FormData) =>
  String(form.get('idempotency_key') ?? '');
export const version = (form: FormData, name = 'expected_version') => {
  const value = Number(form.get(name));
  return Number.isSafeInteger(value) && value > 0 ? value : null;
};
export const dateOrNull = (form: FormData, name: string) =>
  String(form.get(name) ?? '').trim() || null;
export const text = (form: FormData, name: string) =>
  String(form.get(name) ?? '').replaceAll('\r\n', '\n');

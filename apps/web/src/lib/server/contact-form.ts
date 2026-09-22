import type { components } from '$lib/api/schema';

export function contactForm(
  form: FormData,
): components['schemas']['ContactInput'] {
  const identifiers = [
    ...String(form.get('emails') ?? '')
      .split('\n')
      .map((value) => ({ kind: 'email', value: value.trim() })),
    ...String(form.get('phones') ?? '')
      .split('\n')
      .map((value) => ({ kind: 'phone', value: value.trim() })),
  ].filter((item) => item.value);
  return {
    display_name: String(form.get('display_name') ?? ''),
    notes: String(form.get('notes') ?? ''),
    identifiers,
    allow_shared: form.get('allow_shared') === 'on',
  };
}

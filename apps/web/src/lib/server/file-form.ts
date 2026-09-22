/** Keep retry identity and the user's original version across failed writes. */
export function fileDraft(form: FormData) {
  const text = (name: string) => String(form.get(name) ?? '');
  const file = form.get('file');
  return {
    filename: typeof file === 'object' && file ? file.name : text('filename'),
    media_type:
      typeof file === 'object' && file ? file.type : text('media_type'),
    notes: text('notes'),
    owner_membership_id: text('owner_membership_id'),
    key: text('idempotency_key'),
    expected_version: text('expected_version'),
    intent: text('intent'),
  };
}

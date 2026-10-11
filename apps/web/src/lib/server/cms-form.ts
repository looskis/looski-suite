import type { components } from '$lib/api/schema';

export type ContentField = components['schemas']['ContentField'];
export type ContentFieldKind = components['schemas']['ContentFieldKind'];
export const fieldKinds: { kind: ContentFieldKind; label: string }[] = [
  { kind: 'text', label: 'Text' },
  { kind: 'rich_text', label: 'Rich text' },
  { kind: 'number', label: 'Number' },
  { kind: 'boolean', label: 'Yes or no' },
  { kind: 'date', label: 'Date' },
  { kind: 'url', label: 'Web address' },
  { kind: 'media', label: 'Media' },
  { kind: 'form', label: 'Embedded form' },
];
const text = (form: FormData, name: string) =>
  String(form.get(name) ?? '').replaceAll('\r\n', '\n');

/** A stable key derived once from the label; later label edits never change it. */
export function fieldKey(label: string, taken: string[]) {
  const base =
    label
      .normalize('NFKD')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '_')
      .replace(/^[^a-z]+|_+$/g, '')
      .slice(0, 56) || 'field';
  let key = base;
  for (let n = 2; taken.includes(key); n++) key = `${base}_${n}`;
  return key;
}
/** Lowercase letters, digits and single hyphens: the server's address rule. */
export function slugFrom(title: string) {
  return (
    title
      .normalize('NFKD')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .slice(0, 120)
      .replace(/-+$/, '') || 'page'
  );
}
/** Read every row of the type builder as the browser submitted it. */
export function readFields(form: FormData): ContentField[] {
  const fields: ContentField[] = [];
  const count = Number(form.get('field_count') ?? 0);
  for (let i = 0; i < count; i++) {
    fields.push({
      key: text(form, `field.${i}.key`),
      label: text(form, `field.${i}.label`),
      kind: text(form, `field.${i}.kind`) as ContentFieldKind,
      required: form.get(`field.${i}.required`) === 'on',
      help: text(form, `field.${i}.help`),
    });
  }
  return fields;
}
/** Apply the pressed builder button to the submitted rows. */
export function applyOperation(fields: ContentField[], form: FormData) {
  const op = text(form, 'op');
  const [name, at] = op.split(':');
  const index = Number(at);
  if (name === 'add')
    fields.push({
      key: fieldKey(
        text(form, 'new_label'),
        fields.map((f) => f.key),
      ),
      label: text(form, 'new_label'),
      kind: text(form, 'new_kind') as ContentFieldKind,
      required: form.get('new_required') === 'on',
      help: '',
    });
  if (name === 'remove') fields.splice(index, 1);
  if ((name === 'up' && index > 0) || name === 'down') {
    const target = name === 'up' ? index - 1 : index + 1;
    if (fields[target] && fields[index])
      [fields[index], fields[target]] = [fields[target], fields[index]];
  }
  return fields;
}
/** Values for the API, by field kind. A number that does not parse is sent as
 * typed so the server explains it; empty values are removed by the server. */
export function readValues(fields: ContentField[], form: FormData) {
  const values: Record<string, unknown> = {};
  for (const field of fields) {
    const name = `value.${field.key}`;
    if (field.kind === 'boolean') {
      values[field.key] = form.get(name) === 'on';
      continue;
    }
    const value = text(form, name);
    if (!value.trim()) continue;
    if (field.kind === 'number' && Number.isFinite(Number(value)))
      values[field.key] = Number(value);
    else values[field.key] = value;
  }
  return values;
}

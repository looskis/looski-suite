import {
  hasOptions,
  fieldKinds,
  type Field,
  type FieldKind,
} from '$lib/form-definition';

export type Addition = {
  label: string;
  key: string;
  kind: string;
  options: string;
  required: boolean;
};
export type BuilderDraft = {
  name: string;
  description: string;
  confirmation_message: string;
  expected_version: string;
  fields: Field[];
  addition: Addition;
};
const lines = (text: string) =>
  text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
/** A stable identity derived once from the label; later label edits keep it. */
function newKey(label: string, taken: string[], fallback: string) {
  let base = label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 34)
    .replace(/_+$/, '');
  if (!base) base = fallback;
  if (!/^[a-z]/.test(base)) base = `${fallback}_${base}`.slice(0, 34);
  let key = base;
  for (let n = 2; taken.includes(key); n++) key = `${base}_${n}`;
  return key;
}
function newOptions(text: string, field: Field) {
  const options = field.options ?? [];
  for (const label of lines(text))
    options.push({
      label,
      key: newKey(
        label,
        options.map((o) => o.key),
        'option',
      ),
    });
  field.options = options;
}
/** The whole draft is on the page, so every button submits every edit. */
export function builderDraft(form: FormData): BuilderDraft {
  const t = (k: string) => String(form.get(k) ?? '').replaceAll('\r\n', '\n');
  const fields: Field[] = [];
  const count = Math.min(Number(t('field_count')) || 0, 200);
  for (let i = 0; i < count; i++) {
    const at = (k: string) => t(`field.${i}.${k}`);
    const kind = at('kind') as FieldKind;
    const field: Field = {
      key: at('key'),
      label: at('label'),
      kind,
      help: at('help'),
      required: kind !== 'statement' && form.has(`field.${i}.required`),
      options: [],
      show_if: null,
    };
    if (hasOptions(kind)) {
      const options = Math.min(Number(at('option_count')) || 0, 200);
      for (let j = 0; j < options; j++) {
        const label = at(`option.${j}.label`);
        if (label.trim())
          field.options?.push({ key: at(`option.${j}.key`), label });
      }
      newOptions(at('new_options'), field);
    }
    const conditions = form.getAll(`field.${i}.show_if`).map(String);
    if (conditions.length) {
      const [source] = conditions[0].split('=');
      field.show_if = {
        field: source,
        equals: conditions
          .filter((c) => c.startsWith(`${source}=`))
          .map((c) => c.slice(source.length + 1)),
      };
    }
    fields.push(field);
  }
  return {
    name: t('name'),
    description: t('description'),
    confirmation_message: t('confirmation_message'),
    expected_version: t('expected_version'),
    fields,
    addition: {
      label: t('add.label'),
      key: t('add.key'),
      kind: t('add.kind') || 'short_text',
      options: t('add.options'),
      required: form.has('add.required'),
    },
  };
}
/** Conditions may name one source question; several would be silently narrowed. */
function mixedCondition(form: FormData, draft: BuilderDraft) {
  return draft.fields.some((_, i) => {
    const sources = new Set(
      form.getAll(`field.${i}.show_if`).map((c) => String(c).split('=')[0]),
    );
    return sources.size > 1;
  });
}
/** Apply one button's change; the result is what the API validates and stores. */
export function applyOperation(
  form: FormData,
  draft: BuilderDraft,
  operation: string,
): { fields: Field[] } | { message: string } {
  if (mixedCondition(form, draft))
    return {
      message:
        'A question can depend on one earlier question. Choose values from a single question.',
    };
  const fields = structuredClone(draft.fields);
  const addition = draft.addition;
  if (addition.label.trim()) {
    const kind = addition.kind as FieldKind;
    if (!fieldKinds.some((k) => k.value === kind))
      return { message: 'Choose a supported question type' };
    const field: Field = {
      key:
        addition.key.trim() ||
        newKey(
          addition.label,
          fields.map((f) => f.key),
          'question',
        ),
      label: addition.label,
      kind,
      help: '',
      required: kind !== 'statement' && addition.required,
      options: [],
      show_if: null,
    };
    if (hasOptions(kind)) newOptions(addition.options, field);
    fields.push(field);
  } else if (operation === 'add')
    return { message: 'Give the new question a label' };
  const [name, a, b] = operation.split(':');
  const index = Number(a);
  const swap = <T>(list: T[], i: number, j: number) => {
    if (list[i] !== undefined && list[j] !== undefined)
      [list[i], list[j]] = [list[j], list[i]];
  };
  if (name === 'up') swap(fields, index, index - 1);
  else if (name === 'down') swap(fields, index, index + 1);
  else if (name === 'remove') fields.splice(index, 1);
  else if (name === 'remove_option')
    fields[index]?.options?.splice(Number(b), 1);
  else if (name === 'option_up' && fields[index]?.options)
    swap(fields[index].options, Number(b), Number(b) - 1);
  else if (name !== 'save' && name !== 'publish' && name !== 'add')
    return { message: 'Choose a supported change' };
  return { fields };
}

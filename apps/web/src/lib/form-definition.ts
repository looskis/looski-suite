import type { components } from '$lib/api/schema';
export type Definition = components['schemas']['FormDefinition'];
export type Field = components['schemas']['Field'];
export type FieldKind = components['schemas']['FieldKind'];
export type Run = components['schemas']['RunSummary'];
export type AnswerInput = Record<string, string | string[]>;

export const fieldKinds: { value: FieldKind; label: string }[] = [
  { value: 'short_text', label: 'Short text' },
  { value: 'long_text', label: 'Long text' },
  { value: 'email', label: 'Email' },
  { value: 'phone', label: 'Phone' },
  { value: 'number', label: 'Number' },
  { value: 'date', label: 'Date' },
  { value: 'choice', label: 'Choice (pick one)' },
  { value: 'multi_choice', label: 'Multiple choice (pick any)' },
  { value: 'checkbox', label: 'Checkbox' },
  { value: 'statement', label: 'Statement (no answer)' },
];
export const kindLabel = (kind: FieldKind) =>
  fieldKinds.find((k) => k.value === kind)?.label ?? kind;
export const hasOptions = (kind: FieldKind) =>
  kind === 'choice' || kind === 'multi_choice';

function matches(equals: string[], answer: unknown) {
  if (typeof answer === 'string') return equals.includes(answer);
  if (typeof answer === 'boolean') return equals.includes(String(answer));
  if (Array.isArray(answer)) return answer.some((a) => equals.includes(a));
  return false;
}
/** The same rule the API validates with: a shown earlier answer must match. */
export function visibleFields(
  definition: Definition,
  answers: Record<string, unknown>,
) {
  const shown: Field[] = [];
  for (const field of definition.fields) {
    const condition = field.show_if;
    if (
      !condition ||
      (shown.some((f) => f.key === condition.field) &&
        matches(condition.equals, answers[condition.field]))
    )
      shown.push(field);
  }
  return shown;
}
export const answerName = (key: string) => `answer.${key}`;
/**
 * Read browser form data into answers for the visible questions. Without
 * scripting every question is on the page, so answers left in questions that
 * end up hidden are dropped here rather than rejected by the API.
 */
export function readAnswers(definition: Definition, form: FormData) {
  const raw: Record<string, unknown> = {};
  const input: AnswerInput = {};
  const text = (key: string) =>
    String(form.get(answerName(key)) ?? '').replaceAll('\r\n', '\n');
  for (const field of definition.fields) {
    if (field.kind === 'statement') continue;
    if (field.kind === 'multi_choice') {
      const chosen = form.getAll(answerName(field.key)).map(String);
      raw[field.key] = chosen;
      input[field.key] = chosen;
    } else if (field.kind === 'checkbox') {
      raw[field.key] = form.has(answerName(field.key));
      if (raw[field.key]) input[field.key] = 'on';
    } else {
      raw[field.key] = text(field.key);
      input[field.key] = text(field.key);
    }
  }
  const answers: Record<string, unknown> = {};
  const errors: Record<string, string> = {};
  for (const field of visibleFields(definition, raw)) {
    if (field.kind === 'statement') continue;
    const value = raw[field.key];
    const missing =
      value === false ||
      (Array.isArray(value) && value.length === 0) ||
      (typeof value === 'string' && value.trim() === '');
    if (missing && field.required) {
      errors[field.key] = 'Answer this question';
      continue;
    }
    if (field.kind === 'checkbox') answers[field.key] = value;
    else if (missing) continue;
    else if (field.kind === 'number') {
      const number = Number(value);
      if (Number.isFinite(number)) answers[field.key] = number;
      else errors[field.key] = 'Enter a number';
    } else answers[field.key] = value;
  }
  return { answers, input, errors };
}
/** A readable answer using the labels of the version that was answered. */
export function answerText(field: Field | undefined, value: unknown): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  const label = (key: unknown) =>
    field?.options?.find((o) => o.key === key)?.label ?? String(key);
  if (Array.isArray(value)) return value.map(label).join('; ');
  return field && hasOptions(field.kind) ? label(value) : String(value);
}
/** Plain-language state of a recipe run, including failures held on its job. */
export function runStatus(run: Run): {
  label: string;
  detail: string;
  attention: boolean;
} {
  if (run.job_state === 'blocked' || run.job_state === 'exhausted') {
    const reasons: Record<string, string> = {
      authority:
        'The member who activated this recipe no longer has a permission it needs. Restore it or reactivate the recipe, then retry.',
      module_disabled:
        'CRM or Forms is turned off for this workspace. Turn it on, then retry.',
      invalid_input:
        'The mapping does not fit these answers, for example because a mapped question is missing from the version answered.',
      unavailable:
        'A record this run needs, such as the mapped pipeline or stage, is unavailable or changed.',
      database: 'The run failed repeatedly on a storage error. Retry it.',
      provider: 'The run failed repeatedly on a temporary error. Retry it.',
      lease_expired: 'The worker stopped repeatedly during this run. Retry it.',
    };
    return {
      label: 'Failed',
      detail:
        reasons[run.error_code ?? ''] ??
        `This run stopped (${run.error_code ?? run.job_state}). Retry it.`,
      attention: true,
    };
  }
  if (run.job_state === 'cancelled')
    return {
      label: 'Cancelled',
      detail: 'Its background work was cancelled.',
      attention: false,
    };
  switch (run.state) {
    case 'awaiting_review':
      return {
        label: 'Needs review',
        detail:
          'The contact could not be matched automatically. Resolve the intake, then retry to record the inquiry.',
        attention: true,
      };
    case 'completed':
      return {
        label: 'Completed',
        detail: 'The contact and inquiry were recorded.',
        attention: false,
      };
    case 'dismissed':
      return {
        label: 'Dismissed',
        detail: 'A reviewer dismissed the intake, so no inquiry was recorded.',
        attention: false,
      };
    default:
      return {
        label: run.job_state === 'running' ? 'Running' : 'Queued',
        detail: 'Waiting for background work.',
        attention: false,
      };
  }
}
export const canRetry = (run: Run) =>
  run.job_state === 'blocked' ||
  run.job_state === 'exhausted' ||
  (run.state === 'awaiting_review' && run.job_state === 'completed');

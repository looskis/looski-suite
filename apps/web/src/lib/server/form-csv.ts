import { answerText, type Field } from '$lib/form-definition';
/**
 * Spreadsheets evaluate text beginning with these characters, including after
 * leading whitespace and in their full-width forms. A leading apostrophe makes
 * the cell literal text.
 */
const formula = /^[\s 　]*[=+\-@＝＋－＠]|^[\t\r\n]/;
export function textCell(text: string) {
  const safe = formula.test(text) ? `'${text}` : text;
  return /[",\r\n]/.test(safe) ? `"${safe.replaceAll('"', '""')}"` : safe;
}
/** Numeric answers stay numbers: a value such as -5 cannot hold a formula. */
export function answerCell(field: Field | undefined, value: unknown) {
  if (typeof value === 'number' && Number.isFinite(value)) return String(value);
  return textCell(answerText(field, value));
}
export const csvRow = (cells: string[]) => `${cells.join(',')}\r\n`;

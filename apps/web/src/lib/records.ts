import type { components } from '$lib/api/schema';
export type Field = components['schemas']['RecordField'];
export type Filter =
  | { type: 'condition'; field: string; operator: string; value?: unknown }
  | { type: 'group'; operator: string; rules: Filter[] };
export type ViewConfig = {
  columns: string[];
  filter?: Filter | null;
  sort: string;
  direction: string;
  archived?: boolean;
};
export type SavedView = {
  id: string;
  name: string;
  mine: boolean;
  shared: boolean;
  version: number;
  config: ViewConfig;
};
export type RecordRow = {
  id: string;
  display_name?: string;
  name?: string;
  properties?: { [key: string]: unknown };
  identifiers?: { kind: string; value: string }[];
  [key: string]: unknown;
};
export const defaultColumns = (kind: string) =>
  kind === 'contacts'
    ? ['name', 'status', 'job_title', 'primary_email', 'phone', 'updated_at']
    : [
        'name',
        'status',
        'industry',
        'website_url',
        'employee_count',
        'location.locality',
        'location.country_code',
      ];
export function fieldValue(row: RecordRow, key: string): unknown {
  if (key === 'name') return row.display_name ?? row.name;
  if (
    row.identifiers &&
    ['primary_email', 'secondary_email', 'email', 'phone'].includes(key)
  ) {
    const values = row.identifiers
      .filter((i) => i.kind === (key === 'phone' ? 'phone' : 'email'))
      .map((i) => i.value);
    return key === 'primary_email'
      ? values[0]
      : key === 'secondary_email'
        ? values[1]
        : values.join(', ');
  }
  if (['notes', 'website_url', 'created_at', 'updated_at'].includes(key))
    return row[key];
  return key
    .split('.')
    .reduce<unknown>(
      (value, part) =>
        value && typeof value === 'object'
          ? (value as { [key: string]: unknown })[part]
          : undefined,
      row.properties,
    );
}
export function displayValue(value: unknown, field?: Field): string {
  if (value === null || value === undefined || value === '') return '—';
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  if (field?.key === 'location.country_code') {
    try {
      return (
        new Intl.DisplayNames('en', { type: 'region' }).of(String(value)) ??
        String(value)
      );
    } catch {
      return String(value);
    }
  }
  if (field?.field_type === 'date')
    return new Intl.DateTimeFormat('en', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      timeZone: 'UTC',
    }).format(new Date(String(value)));
  if (typeof value === 'number')
    return new Intl.NumberFormat(
      'en-US',
      field?.key === 'annual_revenue'
        ? { style: 'currency', currency: 'USD', maximumFractionDigits: 0 }
        : {},
    ).format(value);
  return String(value);
}
export const operators = (f?: Field) => [
  ['eq', 'is'],
  ['neq', 'is not'],
  ...(['text', 'email', 'url'].includes(f?.field_type ?? 'text')
    ? [
        ['contains', 'contains'],
        ['not_contains', 'does not contain'],
        ['starts_with', 'starts with'],
      ]
    : []),
  ...(['number', 'date'].includes(f?.field_type ?? '')
    ? [
        ['gt', 'is greater than'],
        ['gte', 'is at least'],
        ['lt', 'is less than'],
        ['lte', 'is at most'],
      ]
    : []),
  ['in', 'is any of'],
  ['not_in', 'is none of'],
  ['is_empty', 'is empty'],
  ['is_not_empty', 'is not empty'],
];
export const countries =
  'AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CR CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG EH ER ES ET FI FJ FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TF TG TH TJ TK TL TM TN TO TR TT TV TW TZ UA UG UM US UY UZ VA VC VE VG VI VN VU WF WS YE YT ZA ZM ZW'
    .split(' ')
    .map((code) => ({
      code,
      name: new Intl.DisplayNames('en', { type: 'region' }).of(code) ?? code,
    }))
    .sort((a, b) => a.name.localeCompare(b.name));
export function filterProblem(node: Filter, fields: Field[]): string {
  let count = 0;
  function check(rule: Filter, depth: number): string {
    count++;
    if (count > 60 || depth > 5)
      return 'Use at most 60 conditions and five nesting levels.';
    if (rule.type === 'group') {
      for (const child of rule.rules) {
        const issue = check(child, depth + 1);
        if (issue) return issue;
      }
      return '';
    }
    const field = fields.find((f) => f.key === rule.field);
    if (!field) return 'Choose a field for every condition.';
    if (['is_empty', 'is_not_empty'].includes(rule.operator)) return '';
    const multiple = ['in', 'not_in'].includes(rule.operator);
    const values = multiple
      ? Array.isArray(rule.value)
        ? rule.value
        : []
      : [rule.value];
    if (!values.length || values.length > 50)
      return 'Choose between 1 and 50 values for a membership condition.';
    for (const value of values) {
      if (
        field.field_type === 'number' &&
        (typeof value !== 'number' || !Number.isFinite(value))
      )
        return `Enter a number for ${field.name}.`;
      if (
        field.field_type === 'date' &&
        (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value))
      )
        return `Choose a date for ${field.name}.`;
      if (
        field.field_type === 'select' &&
        !field.options.includes(String(value))
      )
        return `Choose a valid option for ${field.name}.`;
      if (field.field_type === 'boolean' && typeof value !== 'boolean')
        return `Choose Yes or No for ${field.name}.`;
      if (value === undefined || value === null)
        return `Enter a value for ${field.name}.`;
    }
    return '';
  }
  return check(node, 0);
}

/** Shared sizing policy: bounded content widths, manual overrides, then fill spare space. */
export type TableColumn = {
  key: string;
  label: string;
  minWidth?: number;
  maxWidth?: number;
  extraWidth?: number;
  sort?: 'ascending' | 'descending';
};
export const MIN_COLUMN_WIDTH = 88;
export const MAX_COLUMN_WIDTH = 960;
export function columnWidth(column: TableColumn, value: number) {
  return Math.round(
    Math.max(
      column.minWidth ?? MIN_COLUMN_WIDTH,
      Math.min(
        MAX_COLUMN_WIDTH,
        Number.isFinite(value) ? value : MIN_COLUMN_WIDTH,
      ),
    ),
  );
}
export function fitColumns(
  columns: TableColumn[],
  content: Record<string, number>,
  manual: Record<string, number>,
  available: number,
): number[] {
  const widths = columns.map((c) =>
    columnWidth(
      c,
      manual[c.key] ?? Math.min(c.maxWidth ?? 420, content[c.key] ?? 160),
    ),
  );
  const spare = Math.max(
    0,
    Math.floor(available) - widths.reduce((sum, width) => sum + width, 0),
  );
  // User-sized columns remain fixed while automatic columns absorb the free space.
  let elastic = columns
    .map((c, i) => (manual[c.key] === undefined ? i : -1))
    .filter((i) => i >= 0);
  if (!elastic.length) elastic = columns.map((_, i) => i);
  if (elastic.length) {
    const share = Math.floor(spare / elastic.length);
    let remainder = spare % elastic.length;
    for (const i of elastic) widths[i] += share + (remainder-- > 0 ? 1 : 0);
  }
  return widths;
}
export function readColumnWidths(raw: string | null): Record<string, number> {
  try {
    const data = JSON.parse(raw ?? '{}');
    if (!data || typeof data !== 'object' || Array.isArray(data)) return {};
    return Object.fromEntries(
      Object.entries(data).filter(
        ([key, value]) =>
          key.length < 100 &&
          typeof value === 'number' &&
          Number.isFinite(value) &&
          value >= MIN_COLUMN_WIDTH &&
          value <= MAX_COLUMN_WIDTH,
      ),
    ) as Record<string, number>;
  } catch {
    return {};
  }
}

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  fitColumns,
  readColumnWidths,
} from '../apps/web/src/lib/table-layout.ts';
const columns = [
  { key: 'name', label: 'Name', minWidth: 160 },
  { key: 'status', label: 'Status' },
];
test('few columns fill available space', () =>
  assert.equal(
    fitColumns(columns, { name: 180, status: 100 }, {}, 1000).reduce(
      (a, b) => a + b,
      0,
    ),
    1000,
  ));
test('crowded columns use content width and scroll', () =>
  assert.deepEqual(
    fitColumns(columns, { name: 300, status: 150 }, {}, 350),
    [300, 150],
  ));
test('manual widths survive while other columns absorb spare space', () =>
  assert.deepEqual(
    fitColumns(columns, { name: 180, status: 100 }, { name: 400 }, 1000),
    [400, 600],
  ));
test('unbounded values never create huge content columns', () =>
  assert.deepEqual(
    fitColumns(columns, { name: 10000, status: 0 }, {}, 400),
    [420, 88],
  ));
test('invalid persisted data cannot set negative or unbounded widths', () =>
  assert.deepEqual(readColumnWidths('{"a":-2,"b":100000,"c":160,"d":"200"}'), {
    c: 160,
  }));

<script lang="ts" generics="Row extends {id:string}">
  import { tick, untrack, type Snippet } from 'svelte';
  import {
    fitColumns,
    columnWidth,
    readColumnWidths,
    type TableColumn,
  } from '$lib/table-layout';
  let {
    columns,
    rows,
    storageKey,
    header,
    cell,
    empty,
    label = 'Records',
  }: {
    columns: TableColumn[];
    rows: Row[];
    storageKey: string;
    header: Snippet<[TableColumn]>;
    cell: Snippet<[Row, TableColumn]>;
    empty: Snippet;
    label?: string;
  } = $props();
  let viewport: HTMLDivElement;
  let table: HTMLTableElement;
  let available = $state(0);
  let content = $state<Record<string, number>>({});
  let manual = $state<Record<string, number>>({});
  let resizing = $state('');
  const widths = $derived(fitColumns(columns, content, manual, available));
  const total = $derived(widths.reduce((sum, width) => sum + width, 0));
  function persist() {
    try {
      localStorage.setItem(storageKey, JSON.stringify(manual));
    } catch {
      /* Storage can be unavailable in private or embedded contexts. */
    }
  }
  function reset(key: string) {
    const next = { ...manual };
    delete next[key];
    manual = next;
    persist();
  }
  function measure() {
    if (!table) return;
    const canvas = document.createElement('canvas');
    const context = canvas.getContext('2d');
    if (!context) return;
    const measured: Record<string, number> = {};
    for (const [index, column] of columns.entries()) {
      let width = 0;
      for (const element of table.querySelectorAll<HTMLElement>(
        `[data-measure-column="${index}"]`,
      )) {
        const style = getComputedStyle(element);
        context.font = style.font;
        const text = element.textContent?.trim() ?? '';
        const spacing = parseFloat(style.letterSpacing) || 0;
        width = Math.max(
          width,
          ...text
            .split('\n')
            .map(
              (line) =>
                context.measureText(line).width +
                Math.max(0, line.length - 1) * spacing,
            ),
        );
      }
      measured[column.key] = Math.ceil(
        Math.max(
          width + (column.extraWidth ?? 0) + 36,
          column.label.length * 7 + 56,
        ),
      );
    }
    content = measured;
  }
  $effect(() => {
    const key = storageKey;
    try {
      manual = readColumnWidths(localStorage.getItem(key));
    } catch {
      manual = {};
    }
  });
  $effect(() => {
    columns;
    rows;
    let cancelled = false;
    tick().then(() => {
      if (!cancelled) untrack(measure);
    });
    return () => {
      cancelled = true;
    };
  });
  $effect(() => {
    if (!viewport) return;
    const observer = new ResizeObserver(([entry]) => {
      available = Math.floor(entry.contentRect.width);
    });
    observer.observe(viewport);
    document.fonts.ready.then(measure);
    return () => observer.disconnect();
  });
  function start(event: PointerEvent, column: TableColumn, index: number) {
    if (event.button !== 0) return;
    event.preventDefault();
    const handle = event.currentTarget as HTMLElement;
    const startX = event.clientX;
    const startWidth = widths[index];
    resizing = column.key;
    handle.setPointerCapture(event.pointerId);
    const move = (e: PointerEvent) => {
      manual = {
        ...manual,
        [column.key]: columnWidth(column, startWidth + e.clientX - startX),
      };
    };
    const finish = () => {
      resizing = '';
      persist();
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', finish);
      handle.removeEventListener('pointercancel', finish);
      handle.removeEventListener('lostpointercapture', finish);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', finish);
    handle.addEventListener('pointercancel', finish);
    handle.addEventListener('lostpointercapture', finish);
  }
  function resizeKey(event: KeyboardEvent, column: TableColumn, index: number) {
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      manual = {
        ...manual,
        [column.key]: columnWidth(
          column,
          widths[index] +
            (event.key === 'ArrowRight' ? 1 : -1) * (event.shiftKey ? 40 : 10),
        ),
      };
      persist();
    } else if (event.key === 'Home') {
      event.preventDefault();
      reset(column.key);
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex (A scrollable region must be keyboard-focusable.) -->
<div
  bind:this={viewport}
  class="record-table data-table-viewport"
  class:resizing={Boolean(resizing)}
  role="region"
  aria-label={`${label} table, scroll horizontally for more columns`}
  tabindex="0"
>
  <table
    bind:this={table}
    class="data-table"
    style:width={available ? `${total}px` : '100%'}
  >
    <colgroup
      >{#each columns as column, i (column.key)}<col
          style:width={available ? `${widths[i]}px` : undefined}
        />{/each}</colgroup
    >
    <thead
      ><tr
        >{#each columns as column, i (column.key)}<th
            scope="col"
            aria-sort={column.sort}
            ><div class="table-heading" data-measure-column={i}>
              {@render header(column)}
            </div>
            <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (Focusable ARIA separator implements pointer and keyboard resizing.) -->
            <div
              class="column-resizer"
              class:dragging={resizing === column.key}
              role="separator"
              aria-label={`Resize ${column.label} column`}
              aria-orientation="vertical"
              aria-valuenow={widths[i]}
              aria-valuemin={column.minWidth ?? 88}
              aria-valuemax={Math.max(960, widths[i])}
              tabindex="0"
              title="Drag to resize. Arrow keys adjust width. Double-click or Home fits content."
              onpointerdown={(e) => start(e, column, i)}
              onkeydown={(e) => resizeKey(e, column, i)}
              ondblclick={() => reset(column.key)}
            ></div></th
          >{/each}</tr
      ></thead
    >
    <tbody
      >{#each rows as row (row.id)}<tr
          >{#each columns as column, i (column.key)}<td
              ><div class="table-cell" data-measure-column={i}>
                {@render cell(row, column)}
              </div></td
            >{/each}</tr
        >{:else}<tr><td colspan={columns.length}>{@render empty()}</td></tr
        >{/each}</tbody
    >
  </table>
</div>

<style>
  .data-table-viewport {
    width: 100%;
    max-width: 100%;
    min-width: 0;
    overflow: auto;
    contain: inline-size;
    overscroll-behavior-x: contain;
  }
  .data-table-viewport .data-table {
    table-layout: fixed;
    min-width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    margin: 0;
  }
  .data-table th {
    position: relative;
    overflow: visible;
  }
  .table-heading,
  .table-cell {
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .table-heading {
    padding-right: 10px;
  }
  .table-cell :global(.record-name) {
    min-width: 0;
    max-width: 100%;
  }
  .table-cell :global(.record-name > span:last-child) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .column-resizer {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 9px;
    z-index: 3;
    cursor: col-resize;
    touch-action: none;
    outline-offset: -2px;
  }
  .column-resizer:after {
    content: '';
    position: absolute;
    left: 4px;
    top: 25%;
    bottom: 25%;
    width: 1px;
    background: var(--border-default);
  }
  .column-resizer:hover:after,
  .column-resizer:focus-visible:after,
  .column-resizer.dragging:after {
    width: 2px;
    background: var(--accent);
    top: 0;
    bottom: 0;
  }
  .resizing {
    user-select: none;
    cursor: col-resize;
  }
</style>

<script lang="ts">
  import { tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { actionCommands, matchCommands, type Command } from '$lib/commands';
  let {
    root,
    links,
    onTheme,
  }: {
    root: string;
    links: readonly { path: string; label: string }[];
    onTheme: (theme: 'light' | 'dark' | 'system') => void;
  } = $props();
  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let query = $state('');
  let selected = $state(0);
  const commands = $derived<Command[]>([
    ...actionCommands(root),
    ...links.map((link) => ({
      id: `nav:${link.path}`,
      label: link.label,
      keywords: link.path,
      group: 'Navigation' as const,
      href: `${root}/${link.path}`,
    })),
    {
      id: 'workspaces',
      label: 'Switch workspace',
      keywords: 'team account',
      group: 'Navigation',
      href: '/workspaces',
    },
    ...(['light', 'dark', 'system'] as const).map((theme) => ({
      id: `theme:${theme}`,
      label:
        theme === 'system'
          ? 'Use system appearance'
          : `Switch to ${theme} mode`,
      keywords: 'theme appearance',
      group: 'Appearance' as const,
      theme,
    })),
  ]);
  const matches = $derived(matchCommands(commands, query));
  $effect(() => {
    query;
    selected = 0;
  });
  export function open() {
    query = '';
    selected = 0;
    dialog.showModal();
    tick().then(() => input.focus());
  }
  async function run(command: Command) {
    dialog.close();
    if (command.theme) onTheme(command.theme);
    else if (command.href) {
      await goto(command.href);
      window.dispatchEvent(
        new CustomEvent('looski:command', {
          detail: {
            action: command.id.startsWith('create:') ? 'create' : undefined,
          },
        }),
      );
    }
  }
  function shortcut(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      if (dialog.open) dialog.close();
      else open();
    }
  }
  function keydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      selected = matches.length
        ? (selected + (event.key === 'ArrowDown' ? 1 : -1) + matches.length) %
          matches.length
        : 0;
      tick().then(() =>
        dialog
          .querySelector(`#command-${selected}`)
          ?.scrollIntoView({ block: 'nearest' }),
      );
    } else if (event.key === 'Enter' && matches[selected]) {
      event.preventDefault();
      run(matches[selected]);
    }
  }
</script>

<svelte:window onkeydown={shortcut} />
<dialog
  bind:this={dialog}
  class="command-palette"
  aria-label="Commands"
  onclose={() => (query = '')}
>
  <div class="command-search">
    <span aria-hidden="true">⌕</span><input
      bind:this={input}
      bind:value={query}
      onkeydown={keydown}
      role="combobox"
      aria-label="Search pages and actions"
      aria-autocomplete="list"
      aria-expanded="true"
      aria-controls="command-results"
      aria-activedescendant={matches.length ? `command-${selected}` : undefined}
      placeholder="Search pages and actions…"
      autocomplete="off"
    /><button
      type="button"
      class="dismiss"
      onclick={() => dialog.close()}
      aria-label="Close commands">Esc</button
    >
  </div>
  <div
    class="command-results"
    id="command-results"
    role="listbox"
    aria-label="Commands"
  >
    {#each matches as command, i (command.id)}{#if i === 0 || command.group !== matches[i - 1].group}<div
          class="command-group"
          role="presentation"
        >
          {command.group}
        </div>{/if}
      <div
        id={`command-${i}`}
        role="option"
        aria-selected={selected === i}
        tabindex="-1"
        class="command-option"
        onpointermove={() => (selected = i)}
        onclick={() => run(command)}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') run(command);
        }}
      >
        <span class="command-symbol" aria-hidden="true"
          >{command.group === 'Actions'
            ? '+'
            : command.group === 'Appearance'
              ? '◐'
              : '↗'}</span
        ><span>{command.label}</span>{#if selected === i}<kbd aria-hidden="true"
            >↵</kbd
          >{/if}
      </div>{:else}<p class="no-results">
        No matching pages or actions.
      </p>{/each}
  </div>
  <div class="command-footer">
    <span>↑ ↓ to navigate</span><span>↵ to open</span><span>Esc to close</span>
  </div>
</dialog>

<style>
  .command-palette {
    position: fixed;
    inset: 12vh auto auto 50%;
    transform: translateX(-50%);
    width: min(580px, calc(100vw - 32px));
    max-height: 75dvh;
    margin: 0;
    padding: 0;
    color: var(--text-primary);
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: 12px;
    box-shadow: 0 20px 80px #0004;
  }
  .command-palette::backdrop {
    background: #080e1a66;
    backdrop-filter: blur(2px);
  }
  .command-search {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-default);
  }
  .command-search input {
    border: 0;
    background: transparent;
    margin: 0;
    box-shadow: none;
    min-width: 0;
  }
  .command-search > span {
    font-size: 24px;
    color: var(--text-secondary);
  }
  .dismiss {
    background: var(--surface-hover);
    color: var(--text-secondary);
    font-size: 10px;
    padding: 4px 7px;
    min-height: 0;
    white-space: nowrap;
  }
  .command-results {
    max-height: 50dvh;
    overflow: auto;
    padding: 8px;
  }
  .command-group {
    padding: 12px 10px 6px;
    font-size: 10px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .command-option {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
  }
  .command-option[aria-selected='true'] {
    background: var(--accent-subtle);
  }
  .command-option > span:nth-child(2) {
    flex: 1;
  }
  .command-symbol {
    width: 18px;
    color: var(--text-secondary);
    text-align: center;
  }
  .command-option kbd {
    font: inherit;
    color: var(--text-secondary);
  }
  .command-footer {
    display: flex;
    gap: 16px;
    padding: 10px 16px;
    font-size: 10px;
    color: var(--text-secondary);
    border-top: 1px solid var(--border-default);
  }
  .no-results {
    padding: 24px 12px;
    color: var(--text-secondary);
  }
</style>

<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { page } from '$app/state';
  import { createCommands } from '$lib/commands';
  let {
    title,
    label = title,
    children,
    error = false,
    secondary = false,
    message,
  }: {
    title: string;
    label?: string;
    children: Snippet;
    error?: boolean;
    secondary?: boolean;
    message?: string;
  } = $props();
  let dialog: HTMLDialogElement;
  let openedFor = '';
  onMount(() => {
    window.addEventListener('looski:command', commandOpen);
    return () => window.removeEventListener('looski:command', commandOpen);
  });
  function commandOpen() {
    const relative = page.url.pathname.replace(
      `/workspaces/${page.params.workspace}/`,
      '',
    );
    if (
      page.url.searchParams.get('action') === 'create' &&
      createCommands.some(
        ([path, label]) => path === relative && label === title,
      )
    ) {
      dialog?.showModal();
      queueMicrotask(() =>
        dialog
          ?.querySelector<HTMLInputElement>(
            'input:not([type=hidden]),textarea,select',
          )
          ?.focus(),
      );
    }
  }
  $effect(() => {
    const url = page.url.href;
    if (url !== openedFor) {
      openedFor = url;
      commandOpen();
    }
  });
  $effect(() => {
    if (error && dialog && !dialog.open) dialog.showModal();
  });
</script>

<button type="button" class:secondary onclick={() => dialog.showModal()}
  >{label}</button
>
<dialog bind:this={dialog} class="record-drawer" aria-label={title}>
  <div class="drawer-heading">
    <h2>{title}</h2>
    <button
      type="button"
      class="drawer-close"
      aria-label="Close dialog"
      onclick={() => dialog.close()}>✕</button
    >
  </div>
  <div class="drawer-body">
    {#if message}<p role="alert" class="error">
        {message}
      </p>{/if}{@render children()}
  </div>
</dialog>

<style>
  .record-drawer {
    position: fixed;
    inset: 0 0 0 auto;
    margin: 0;
    max-width: min(560px, 100%);
    width: 560px;
    max-height: 100dvh;
    height: 100dvh;
    padding: 0;
    border: 0;
    border-left: 1px solid var(--border-default);
    background: var(--surface-panel);
    color: var(--text-primary);
    box-shadow: -12px 0 60px #0002;
  }
  .record-drawer::backdrop {
    background: #080e1a66;
    backdrop-filter: blur(2px);
  }
  .drawer-heading {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--surface-panel);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 24px;
    border-bottom: 1px solid var(--border-default);
  }
  h2 {
    margin: 0;
    font-size: 1.125rem;
  }
  .drawer-close {
    color: var(--text-secondary);
    background: transparent;
    padding: 8px 12px;
  }
  .drawer-body {
    padding: 24px;
  }
</style>

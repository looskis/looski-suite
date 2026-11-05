<script lang="ts">
  import AccountMenu from '$lib/components/AccountMenu.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import {
    provideAppearance,
    savePreference,
    type Theme,
    type Density,
  } from '$lib/appearance.svelte';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  let search = $state('');
  let theme = $derived<Theme>(data.appearance.theme);
  let density = $derived<Density>(data.appearance.density);
  provideAppearance({
    get theme() {
      return theme;
    },
    get density() {
      return density;
    },
    setTheme(value) {
      theme = value;
      savePreference('looski_theme', value);
    },
    setDensity(value) {
      density = value;
      savePreference('looski_density', value);
    },
  });
  const filtered = $derived(
    data.workspaces.filter((workspace) =>
      workspace.name.toLowerCase().includes(search.trim().toLowerCase()),
    ),
  );
  const shortcuts = [
    { path: 'contacts', label: 'Contacts', icon: 'people' },
    { path: 'tasks', label: 'Tasks', icon: 'tasks' },
    { path: 'projects', label: 'Projects', icon: 'board' },
    { path: 'files', label: 'Files', icon: 'files' },
  ] as const;
</script>

<svelte:head
  ><title>Workspaces · Looski Suite</title><meta
    name="robots"
    content="noindex"
  /></svelte:head
>
<div
  class="app-frame workspace-picker"
  data-theme={theme}
  data-density={density}
>
  <header>
    <a class="suite-brand" href="/"
      ><img src="/brand/looski-mark.svg" width="26" height="26" alt="" /><strong
        >looski</strong
      ><span>suite</span></a
    ><a href="https://www.loo.ski" target="_blank" rel="noopener"
      >About Looski ↗</a
    >
  </header>
  <main id="workspace-list" class="workspace-app">
    <div class="page-heading">
      <h1>Workspaces <span class="count">{data.workspaces.length}</span></h1>
      {#if data.workspaces.length > 1}<label class="workspace-search"
          ><span class="sr-only">Find a workspace</span><input
            type="search"
            placeholder="Find a workspace…"
            bind:value={search}
          /></label
        >{/if}
    </div>
    {#if data.pendingInvitation}<aside class="invitation">
        <Icon name="people" /><span>You have a pending invitation.</span><a
          href="/invitations/accept">Review invitation →</a
        >
      </aside>{/if}
    <div class="workspace-grid">
      {#each filtered as workspace (workspace.id)}
        <article class="workspace-card">
          <a
            class="workspace-open"
            href={`/workspaces/${workspace.id}/contacts`}
            aria-label={`Open ${workspace.name}`}
            ><span class="workspace-avatar"
              >{workspace.name.slice(0, 1).toUpperCase()}</span
            >
            <h2>{workspace.name}</h2>
            <Icon name="arrow" size={18} /></a
          >
          <nav aria-label={`${workspace.name} shortcuts`}>
            {#each shortcuts as shortcut}<a
                href={`/workspaces/${workspace.id}/${shortcut.path}`}
                ><Icon name={shortcut.icon} size={15} />{shortcut.label}</a
              >{/each}
          </nav>
        </article>
      {:else}
        <div class="empty">
          <Icon name="building" size={28} />
          <h2>
            {data.workspaces.length
              ? 'No matching workspaces'
              : 'No workspace access yet'}
          </h2>
          <p>
            {data.workspaces.length
              ? 'Try a different name.'
              : 'Ask a workspace administrator for an invitation.'}
          </p>
          {#if data.workspaces.length}<button
              class="secondary"
              onclick={() => (search = '')}>Clear search</button
            >{/if}
        </div>
      {/each}
    </div>
  </main>
  <AccountMenu
    email={data.accountEmail ?? 'Signed-in account'}
    workspaceRoot={data.workspaces[0]
      ? `/workspaces/${data.workspaces[0].id}`
      : undefined}
  />
</div>

<style>
  .workspace-picker {
    display: block;
    min-height: 100dvh;
    background: var(--surface-navigation);
    color: var(--text-primary);
  }
  header {
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 32px;
    border-bottom: 1px solid var(--border-default);
    background: var(--surface-panel);
  }
  header > a:last-child {
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-decoration: none;
  }
  .suite-brand {
    display: flex;
    align-items: center;
    gap: 8px;
    text-decoration: none;
    color: var(--text-primary);
  }
  .suite-brand strong {
    font-family: 'Space Grotesk', var(--font-family-sans);
    font-size: 1.375rem;
    letter-spacing: -0.06em;
  }
  .suite-brand span {
    font-size: 0.6875rem;
    color: var(--text-secondary);
    border: 1px solid var(--border-default);
    padding: 1px 5px;
    border-radius: 4px;
  }
  main {
    max-width: 1040px;
    margin: 0 auto;
    padding: 56px 32px 100px;
  }
  .workspace-search {
    margin: 0;
    width: 260px;
  }
  .workspace-search input {
    margin: 0;
  }
  .workspace-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 18px;
  }
  .workspace-card {
    border: 1px solid var(--border-default);
    border-radius: 10px;
    overflow: hidden;
    background: var(--surface-panel);
  }
  .workspace-card:hover {
    border-color: var(--border-hover);
  }
  .workspace-open {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 24px;
    text-decoration: none;
    color: var(--text-primary);
  }
  .workspace-open h2 {
    flex: 1;
    margin: 0;
  }
  .workspace-avatar {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    background: var(--accent-subtle);
    color: var(--accent);
    font-size: 1rem;
    border-radius: 8px;
  }
  .workspace-card nav {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    border-top: 1px solid var(--border-default);
    padding: 12px;
    gap: 4px;
  }
  .workspace-card nav a {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: 0.6875rem;
    padding: 8px 4px;
    color: var(--text-secondary);
    text-decoration: none;
    border-radius: 5px;
  }
  .workspace-card nav a:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .invitation {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    padding: 16px;
    margin-bottom: 20px;
    border: 1px solid var(--border-accent);
    border-radius: 8px;
    background: var(--accent-subtle);
    font-size: 0.8125rem;
  }
  .invitation span {
    flex: 1;
  }
  .empty {
    grid-column: 1 / -1;
    padding: 50px 20px;
    border: 1px dashed var(--border-input);
    border-radius: 10px;
    text-align: center;
    color: var(--text-secondary);
  }
  .empty p {
    font-size: 0.8125rem;
  }
  @media (max-width: 800px) {
    .workspace-grid {
      grid-template-columns: 1fr;
    }
    main {
      padding: 32px 16px 90px;
    }
    header {
      padding: 0 16px;
    }
  }
</style>

<script lang="ts">
  import { page } from '$app/state';
  import Icon from './Icon.svelte';
  import AccountMenu from './AccountMenu.svelte';
  import CommandPalette from './CommandPalette.svelte';
  import {
    provideAppearance,
    savePreference,
    type Theme,
    type Density,
  } from '$lib/appearance.svelte';
  let { children } = $props();
  let theme = $state<Theme>(page.data.appearance?.theme ?? 'system');
  let density = $state<Density>(page.data.appearance?.density ?? 'compact');
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
  let palette: CommandPalette;
  let mobileOpen = $state(false);
  const root = $derived(`/workspaces/${page.params.workspace}`);
  const email = $derived(page.data.accountEmail ?? 'Signed-in account');
  const allGroups = [
    {
      label: 'Relationships',
      links: [
        { path: 'contacts', label: 'Contacts', icon: 'people' },
        { path: 'visitors', label: 'Visitors', icon: 'activity' },
        { path: 'organizations', label: 'Organizations', icon: 'building' },
        { path: 'crm/opportunities', label: 'Opportunities', icon: 'activity' },
        { path: 'crm/deals', label: 'Deals', icon: 'board' },
        { path: 'communications', label: 'Inbox', icon: 'inbox' },
      ],
    },
    {
      label: 'Work',
      links: [
        { path: 'tasks', label: 'Tasks', icon: 'tasks' },
        { path: 'projects', label: 'Projects', icon: 'board' },
        { path: 'scheduling', label: 'Scheduling', icon: 'calendar' },
        { path: 'time', label: 'Time tracking', icon: 'clock' },
        { path: 'helpdesk', label: 'Helpdesk', icon: 'access' },
      ],
    },
    {
      label: 'Resources',
      links: [
        { path: 'files', label: 'Files', icon: 'files' },
        { path: 'forms', label: 'Forms', icon: 'form' },
        { path: 'cms', label: 'Content', icon: 'content' },
        { path: 'knowledge', label: 'Knowledge base', icon: 'book' },
      ],
    },
    {
      label: 'Manage',
      links: [
        { path: 'intakes', label: 'Intake review', icon: 'inbox' },
        { path: 'tags', label: 'Tags', icon: 'tag' },
        { path: 'access', label: 'Workspace access', icon: 'access' },
        { path: 'crm', label: 'CRM configuration', icon: 'grid' },
        { path: 'jobs', label: 'Background jobs', icon: 'activity' },
        { path: 'exports', label: 'Exports', icon: 'export' },
        { path: 'settings', label: 'Settings', icon: 'grid' },
      ],
    },
  ] as const;
  const groups = $derived(
    page.data.writing?.contributor_only
      ? ([
          {
            label: 'Writing',
            links: [
              { path: 'cms', label: 'My articles', icon: 'content' },
              { path: 'cms/media', label: 'My uploads', icon: 'files' },
              { path: 'cms/contributors', label: 'My profile', icon: 'people' },
              { path: 'cms/activity', label: 'Updates', icon: 'inbox' },
            ],
          },
        ] as const)
      : allGroups,
  );
  const links = $derived(groups.flatMap((group) => [...group.links]));
  const active = $derived(
    [...links]
      .sort((a, b) => b.path.length - a.path.length)
      .find(
        (link) =>
          page.url.pathname === `${root}/${link.path}` ||
          page.url.pathname.startsWith(`${root}/${link.path}/`),
      ),
  );
  function shortcut(event: KeyboardEvent) {
    if (event.key === 'Escape') mobileOpen = false;
  }
  $effect(() => {
    page.url.pathname;
    mobileOpen = false;
  });
</script>

<svelte:window onkeydown={shortcut} />
<div class="app-frame" data-theme={theme} data-density={density}>
  <a class="skip-link" href="#workspace-content">Skip to content</a>
  <aside class="sidebar" class:mobile-open={mobileOpen}>
    <a href="/workspaces" class="suite-brand"
      ><img src="/brand/looski-mark.svg" alt="" width="26" height="26" /><strong
        >looski</strong
      ><span>suite</span></a
    >
    <a class="workspace-switch" href="/workspaces"
      ><span class="workspace-avatar"
        >{(page.data.workspace?.name ?? 'W').slice(0, 1).toUpperCase()}</span
      ><span class="workspace-name"
        >{page.data.workspace?.name ?? 'Workspace'}</span
      ><span aria-hidden="true">⌃⌄</span></a
    >
    <nav aria-label="Workspace navigation">
      {#each groups as group}
        <div class="nav-group">
          <p>{group.label}</p>
          {#each group.links as link}
            <a
              href={`${root}/${link.path}`}
              aria-current={active?.path === link.path ? 'page' : undefined}
              onclick={() => (mobileOpen = false)}
              ><Icon name={link.icon} size={16} /><span>{link.label}</span></a
            >
          {/each}
        </div>
      {/each}
    </nav>
    <AccountMenu {email} workspaceRoot={root} docked />
  </aside>
  <div class="app-body">
    <div class="app-topbar">
      <button
        type="button"
        class="mobile-toggle"
        aria-label="Toggle navigation"
        aria-expanded={mobileOpen}
        onclick={() => (mobileOpen = !mobileOpen)}>☰</button
      >
      <button
        class="mobile-commands"
        type="button"
        aria-label="Open commands"
        onclick={() => palette.open()}>⌕</button
      >
    </div>
    <main id="workspace-content" class="workspace-app" tabindex="-1">
      {@render children()}
    </main>
  </div>

  <CommandPalette
    bind:this={palette}
    {root}
    {links}
    onTheme={(value) => {
      theme = value;
      savePreference('looski_theme', value);
    }}
  />
</div>

<style>
  .app-frame {
    display: grid;
    grid-template-columns: 216px minmax(0, 1fr);
    min-height: 100dvh;
    background: var(--surface-canvas);
    color: var(--text-primary);
    font-size: 0.8125rem;
  }
  .sidebar {
    position: sticky;
    top: 0;
    height: 100dvh;
    display: flex;
    flex-direction: column;
    background: var(--surface-navigation);
    border-right: 1px solid var(--border-default);
    padding: 20px 12px 12px;
  }
  .suite-brand {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
    text-decoration: none;
    padding: 0 10px 20px;
  }
  .suite-brand strong {
    font-family: 'Space Grotesk', var(--font-family-sans);
    font-size: 1.375rem;
    letter-spacing: -0.06em;
  }
  .suite-brand > span {
    font-size: 0.6875rem;
    color: var(--text-secondary);
    border: 1px solid var(--border-default);
    padding: 1px 5px;
    border-radius: 4px;
    margin-top: 3px;
  }
  .workspace-switch {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 9px;
    border: 1px solid var(--border-default);
    border-radius: 7px;
    color: var(--text-primary);
    text-decoration: none;
    margin-bottom: 18px;
    background: var(--surface-panel);
  }
  .workspace-avatar {
    flex: 0 0 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--accent);
    background: var(--accent-subtle);
    font-size: 0.6875rem;
    font-weight: 600;
  }
  .workspace-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    font-size: 0.75rem;
    font-weight: 550;
  }
  .workspace-switch > span:last-child {
    color: var(--text-secondary);
    font-size: 0.5625rem;
  }
  .sidebar > nav {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: thin;
  }
  .nav-group {
    margin-bottom: 16px;
  }
  .nav-group p {
    margin: 0 10px 5px;
    color: var(--text-secondary);
    font-size: 0.625rem;
    letter-spacing: 0.055em;
    font-weight: 550;
  }
  .nav-group a {
    display: flex;
    align-items: center;
    gap: 9px;
    color: var(--text-secondary);
    padding: 7px 10px;
    margin: 2px 0;
    text-decoration: none;
    border-radius: 5px;
    font-size: 0.75rem;
  }
  .nav-group a:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .nav-group a[aria-current='page'] {
    color: var(--accent);
    background: var(--accent-subtle);
    font-weight: 600;
  }

  .app-body {
    min-width: 0;
  }
  .app-topbar {
    height: 52px;
    border-bottom: 1px solid var(--border-default);
    padding: 0 28px;
    display: none;
    align-items: center;
    justify-content: flex-end;
    gap: 16px;
    background: var(--surface-panel);
    font-size: 0.75rem;
  }

  .workspace-app {
    width: 100%;
    margin: 0 auto;
    padding: 26px 28px 90px;
    outline: none;
    overflow-wrap: anywhere;
  }
  .mobile-commands {
    background: transparent;
    color: var(--text-secondary);
    font-size: 22px;
    padding: 4px 8px;
  }
  .mobile-toggle {
    display: none;
  }
  @media (max-width: 800px) {
    .app-frame {
      display: block;
    }
    .sidebar {
      display: none;
      position: fixed;
      z-index: 15;
      width: min(280px, calc(100vw - 60px));
      box-shadow: 16px 0 60px #0003;
      top: 52px;
      height: calc(100dvh - 52px);
    }
    .sidebar.mobile-open {
      display: flex;
    }
    .mobile-toggle {
      display: block;
      min-height: 32px;
      padding: 4px 8px;
      color: var(--text-secondary);
      background: transparent;
    }
    .app-topbar {
      display: flex;
      padding: 0 12px;
      gap: 10px;
    }
  }
  @media print {
    .sidebar,
    .app-topbar,
    :global(.account-avatar),
    :global(.account-menu) {
      display: none;
    }
    .app-frame {
      display: block;
    }
    .workspace-app {
      padding: 0;
    }
  }

  @media (max-width: 800px) {
    .app-topbar {
      justify-content: space-between;
    }
  }
</style>

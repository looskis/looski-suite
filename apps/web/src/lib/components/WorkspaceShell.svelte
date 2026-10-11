<script lang="ts">
  import { page } from '$app/state';
  import Icon from './Icon.svelte';
  let { children } = $props();
  const root = $derived(`/workspaces/${page.params.workspace}`);
  const groups = [
    {
      label: 'Workspace',
      links: [
        { path: 'contacts', label: 'Contacts', icon: 'people' },
        { path: 'organizations', label: 'Organizations', icon: 'building' },
        { path: 'tasks', label: 'Tasks', icon: 'tasks' },
        { path: 'projects', label: 'Projects', icon: 'board' },
        { path: 'files', label: 'Files', icon: 'files' },
        { path: 'forms', label: 'Forms', icon: 'form' },
        { path: 'cms', label: 'Content', icon: 'content' },
        { path: 'knowledge', label: 'Knowledge base', icon: 'book' },
        { path: 'scheduling', label: 'Scheduling', icon: 'calendar' },
        { path: 'time', label: 'Time tracking', icon: 'clock' },
        { path: 'helpdesk', label: 'Helpdesk', icon: 'inbox' },
        { path: 'crm', label: 'Applications', icon: 'grid' },
      ],
    },
    {
      label: 'Manage',
      links: [
        { path: 'intakes', label: 'Intake review', icon: 'inbox' },
        { path: 'tags', label: 'Tags', icon: 'tag' },
        { path: 'access', label: 'Workspace access', icon: 'access' },
        { path: 'jobs', label: 'Background work', icon: 'activity' },
        { path: 'exports', label: 'Export data', icon: 'export' },
      ],
    },
  ] as const;
</script>

<a class="skip-link" href="#workspace-content">Skip to content</a>
<div class="app-frame">
  <aside class="sidebar">
    <a href="/workspaces" class="brand"
      ><span class="brand-mark">l</span>looski<span class="suite">suite</span
      ></a
    >
    <a class="workspace-switch" href="/workspaces"
      ><span class="workspace-avatar"
        >{(page.data.workspace?.name ?? 'W').slice(0, 1).toUpperCase()}</span
      ><span class="workspace-name"
        >{page.data.workspace?.name ?? 'Your workspace'}<small
          >Switch workspace</small
        ></span
      ><span aria-hidden="true">⌄</span></a
    >
    <nav aria-label="Workspace navigation">
      {#each groups as group}
        <div class="nav-group">
          <p>{group.label}</p>
          {#each group.links as link}
            {@const active =
              page.url.pathname === `${root}/${link.path}` ||
              page.url.pathname.startsWith(`${root}/${link.path}/`)}
            <a
              href={`${root}/${link.path}`}
              aria-current={active ? 'page' : undefined}
              ><Icon name={link.icon} /><span>{link.label}</span></a
            >
          {/each}
        </div>
      {/each}
    </nav>
    <div class="sidebar-footer">
      <span class="local-dot"></span>Your business. Your space.
    </div>
  </aside>
  <div class="app-body">
    <div class="app-topbar">
      <span
        >Workspace <span class="divider">/</span>
        <strong
          >{groups
            .map(
              (g) =>
                g.links.find((l) =>
                  page.url.pathname.startsWith(`${root}/${l.path}`),
                )?.label,
            )
            .find(Boolean) ?? 'Overview'}</strong
        ></span
      >
      <form method="POST" action="/logout">
        <button class="sign-out"
          ><Icon name="logout" size={16} />Sign out</button
        >
      </form>
    </div>
    <main id="workspace-content" class="workspace-app" tabindex="-1">
      {@render children()}
    </main>
  </div>
</div>

<style>
  .app-frame {
    display: grid;
    grid-template-columns: 232px minmax(0, 1fr);
    min-height: 100dvh;
  }
  .sidebar {
    position: sticky;
    top: 0;
    height: 100dvh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    background: #192f2c;
    color: #d5e2dc;
    padding: 28px 16px 18px;
  }
  .brand {
    color: #fff;
    padding: 0 12px;
    margin-bottom: 30px;
    font-size: 25px;
  }
  .brand-mark {
    background: #b6dcc3;
    color: #192f2c;
  }
  .suite {
    font-weight: 400;
    font-size: 12px;
    letter-spacing: 0.01em;
    color: #acc1b9;
    align-self: end;
    margin-bottom: 4px;
    margin-left: -5px;
  }
  .workspace-switch {
    display: flex;
    gap: 10px;
    align-items: center;
    color: #e9f0ec;
    text-decoration: none;
    padding: 12px 10px;
    border: 1px solid #ffffff20;
    border-radius: 9px;
    margin-bottom: 22px;
  }
  .workspace-switch:hover {
    background: #ffffff08;
  }
  .workspace-avatar {
    display: grid;
    place-items: center;
    background: #38524b;
    border: 1px solid #ffffff12;
    border-radius: 7px;
    width: 32px;
    height: 32px;
    flex: 0 0 auto;
    font-size: 13px;
  }
  .workspace-name {
    font-size: 12px;
    font-weight: 600;
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .workspace-name small {
    display: block;
    color: #a9beb5;
    font-size: 10px;
    font-weight: 400;
    margin-top: 4px;
  }
  .nav-group {
    margin-bottom: 22px;
  }
  .nav-group p {
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #9cb5ab;
    margin: 0 12px 8px;
  }
  .nav-group a {
    display: flex;
    align-items: center;
    gap: 11px;
    color: #bed0c8;
    padding: 10px 12px;
    margin: 3px 0;
    text-decoration: none;
    border-radius: 7px;
    font-size: 13px;
    transition:
      background 140ms,
      color 140ms;
  }
  .nav-group a:hover {
    background: #ffffff0c;
    color: white;
  }
  .nav-group a[aria-current='page'] {
    background: #d8eede;
    color: #183d30;
    font-weight: 650;
  }
  .sidebar-footer {
    margin-top: auto;
    padding: 20px 12px 0;
    border-top: 1px solid #ffffff12;
    color: #a9beb5;
    font-size: 10px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .local-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #9fcaaa;
  }
  .app-body {
    min-width: 0;
  }
  .app-topbar {
    min-height: 70px;
    background: white;
    border-bottom: 1px solid var(--line);
    padding: 12px 40px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    color: var(--muted);
    font-size: 12px;
  }
  .app-topbar strong {
    font-weight: 550;
    color: var(--ink);
  }
  .divider {
    margin: 0 14px;
    color: #c5ced1;
  }
  .sign-out {
    min-height: 34px;
    padding: 6px 8px;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
  }
  .sign-out:hover {
    background: var(--canvas);
    color: var(--ink);
    box-shadow: none;
  }
  .workspace-app {
    overflow-wrap: anywhere;
    width: 100%;
    max-width: 1360px;
    margin: auto;
    padding: 32px 40px 80px;
    outline: none;
  }
  :global(.workspace-app > header) {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px 20px;
    padding-bottom: 20px;
    margin-bottom: 24px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
  }
  :global(.workspace-app > header a) {
    color: var(--muted);
    text-decoration: none;
  }
  :global(.workspace-app > header a:hover) {
    color: var(--accent);
    text-decoration: underline;
  }
  :global(.workspace-app > header span) {
    color: var(--muted);
  }
  :global(.workspace-app h1) {
    margin: 0 0 10px;
  }
  :global(.workspace-app > p) {
    color: var(--muted);
    font-size: 14px;
    max-width: 76ch;
  }
  :global(.workspace-app section) {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 24px;
    margin-top: 24px;
    box-shadow: var(--shadow);
    min-width: 0;
  }
  :global(.workspace-app section > h2:first-child) {
    margin-top: 0;
  }
  :global(.workspace-app section > p) {
    color: var(--muted);
    font-size: 14px;
  }
  :global(.workspace-app section nav) {
    line-height: 2.2;
    font-size: 13px;
  }
  :global(.workspace-app .toolbar) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 18px;
  }
  :global(.workspace-app .toolbar h2) {
    margin: 0;
  }
  :global(.workspace-app .toolbar a) {
    font-size: 12px;
  }
  :global(.workspace-app .columns) {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 24px;
  }
  :global(.workspace-app .check) {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    font-weight: 400;
    color: var(--muted);
  }
  :global(.workspace-app .check input) {
    margin-top: 2px;
  }
  :global(.workspace-app .inline) {
    display: flex;
    align-items: end;
    flex-wrap: wrap;
    gap: 12px;
    margin: 18px 0;
  }
  :global(.workspace-app .inline label) {
    flex: 1 1 160px;
    margin: 0;
  }
  :global(.workspace-app .inline button) {
    flex: 0 0 auto;
  }
  :global(.workspace-app .table-wrap) {
    overflow-x: auto;
    border: 1px solid var(--line);
    border-radius: 8px;
    margin-top: 18px;
  }
  :global(.workspace-app table) {
    border-collapse: collapse;
    width: 100%;
    text-align: left;
    font-size: 13px;
  }
  :global(.workspace-app th) {
    background: #f7f9fa;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
  }
  :global(.workspace-app th),
  :global(.workspace-app td) {
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
    overflow-wrap: anywhere;
  }
  :global(.workspace-app tbody tr:last-child td) {
    border-bottom: 0;
  }
  :global(.workspace-app tbody tr:hover) {
    background: #f8fbf9;
  }
  :global(.workspace-app td a) {
    font-weight: 550;
    text-decoration: none;
  }
  :global(.workspace-app td a:hover) {
    text-decoration: underline;
  }
  :global(.workspace-app td[colspan]) {
    text-align: center;
    padding: 44px 20px;
    color: var(--muted);
  }
  :global(.workspace-app fieldset) {
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 18px;
    margin: 20px 0;
    min-width: 0;
  }
  :global(.workspace-app legend) {
    padding: 0 8px;
    font-size: 13px;
    font-weight: 600;
  }
  :global(.workspace-app pre) {
    overflow-x: auto;
    background: var(--canvas);
    border-radius: 8px;
    padding: 16px;
    font-size: 12px;
  }
  :global(.workspace-app details) {
    padding: 16px 0;
    border-top: 1px solid var(--line);
  }
  :global(.workspace-app summary) {
    cursor: pointer;
    font-weight: 550;
    font-size: 14px;
  }
  @media (max-width: 1000px) {
    .app-frame {
      grid-template-columns: 200px minmax(0, 1fr);
    }
    .workspace-app {
      padding: 28px 24px 60px;
    }
    .app-topbar {
      padding-inline: 24px;
    }
  }
  @media (max-width: 720px) {
    .app-frame {
      display: block;
    }
    .sidebar {
      position: static;
      height: auto;
      padding: 16px;
      overflow: visible;
    }
    .brand {
      margin-bottom: 16px;
      padding: 0;
      font-size: 22px;
    }
    .workspace-switch,
    .sidebar-footer,
    .nav-group p {
      display: none;
    }
    nav {
      scrollbar-width: thin;
      scrollbar-color: #789b8c transparent;
      display: flex;
      gap: 6px;
      overflow-x: auto;
      padding-bottom: 4px;
    }
    .nav-group {
      display: flex;
      gap: 4px;
      margin: 0;
    }
    .nav-group a {
      min-height: 44px;
      white-space: nowrap;
      margin: 0;
      padding: 9px 11px;
    }
    .app-topbar {
      min-height: 56px;
      padding: 10px 20px;
    }
    .workspace-app {
      padding: 24px 18px 60px;
    }
    :global(.workspace-app section) {
      padding: 20px 16px;
    }
    :global(.workspace-app .columns) {
      grid-template-columns: 1fr;
      gap: 0;
    }
    :global(.workspace-app .inline button) {
      flex-grow: 1;
    }
  }
  nav::-webkit-scrollbar {
    height: 4px;
  }
  nav::-webkit-scrollbar-thumb {
    background: #789b8c;
    border-radius: 4px;
  }
  nav::-webkit-scrollbar-track {
    background: transparent;
  }
  @media print {
    .sidebar,
    .app-topbar {
      display: none;
    }
    .app-frame {
      display: block;
    }
    .workspace-app {
      padding: 0;
    }
    :global(.workspace-app section) {
      box-shadow: none;
      break-inside: avoid;
    }
  }
</style>

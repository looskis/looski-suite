<script lang="ts">
  import { page } from '$app/state';
  const root = $derived(
    `/workspaces/${page.params.workspace}/projects/${page.params.project}`,
  );
  const tabs = [
    { path: '', label: 'Board' },
    { path: '/delivery', label: 'Delivery' },
    { path: '/members', label: 'Members' },
    { path: '/settings', label: 'Settings' },
  ];
</script>

<nav class="project-tabs" aria-label="Project sections">
  {#each tabs as tab}
    {@const active = tab.path
      ? page.url.pathname.startsWith(root + tab.path)
      : page.url.pathname === root ||
        page.url.pathname.startsWith(`${root}/cards`)}
    <a href={root + tab.path} aria-current={active ? 'page' : undefined}
      >{tab.label}</a
    >
  {/each}
</nav>

<style>
  .project-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    margin-bottom: 20px;
    font-size: 13px;
  }
  .project-tabs a {
    text-decoration: none;
    color: var(--muted);
    padding: 6px 0;
  }
  .project-tabs a[aria-current='page'] {
    color: var(--accent);
    font-weight: 650;
    border-bottom: 2px solid var(--accent);
  }
</style>

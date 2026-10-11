<script lang="ts">
  import { page } from '$app/state';
  const root = $derived(`/workspaces/${page.params.workspace}/cms`);
  const tabs = [
    { path: '', label: 'Entries' },
    { path: '/types', label: 'Content types' },
    { path: '/media', label: 'Media' },
  ];
</script>

<nav aria-label="Content sections" class="toolbar">
  {#each tabs as tab}
    {@const active = tab.path
      ? page.url.pathname.startsWith(root + tab.path)
      : page.url.pathname === root ||
        page.url.pathname.startsWith(root + '/entries')}
    <a href={root + tab.path} aria-current={active ? 'page' : undefined}
      >{tab.label}</a
    >
  {/each}
</nav>

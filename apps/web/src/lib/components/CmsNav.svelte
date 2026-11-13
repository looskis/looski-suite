<script lang="ts">
  import { page } from '$app/state';
  const root = $derived(`/workspaces/${page.params.workspace}/cms`);
  const tabs = $derived([
    {
      path: '',
      label: page.data.writing?.contributor_only ? 'My articles' : 'Entries',
    },
    ...(page.data.writing?.can_manage_types
      ? [{ path: '/types', label: 'Content types' }]
      : []),
    { path: '/media', label: 'Media' },
    { path: '/activity', label: 'Updates' },
    ...(page.data.writing?.contributor_only ||
    page.data.writing?.can_manage_contributors
      ? [
          {
            path: '/contributors',
            label: page.data.writing?.contributor_only
              ? 'My profile'
              : 'Contributors',
          },
        ]
      : []),
  ]);
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

<script lang="ts">
  import { page } from '$app/state';
  import SectionNav from './ui/SectionNav.svelte';
  const root = $derived(
    `/workspaces/${page.params.workspace}/projects/${page.params.project}`,
  );
  const tabs = [
    { path: '', label: 'Board' },
    { path: '/delivery', label: 'Delivery' },
    { path: '/members', label: 'Members' },
    { path: '/settings', label: 'Settings' },
  ];
  const items = $derived(
    tabs.map((tab) => ({
      href: root + tab.path,
      label: tab.label,
      active: tab.path
        ? page.url.pathname.startsWith(root + tab.path)
        : page.url.pathname === root ||
          page.url.pathname.startsWith(`${root}/cards`),
    })),
  );
</script>

<SectionNav label="Project sections" {items} />

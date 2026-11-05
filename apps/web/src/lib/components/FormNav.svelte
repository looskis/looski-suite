<script lang="ts">
  import { page } from '$app/state';
  import SectionNav from './ui/SectionNav.svelte';
  const root = $derived(
    `/workspaces/${page.params.workspace}/forms/${page.params.form}`,
  );
  const tabs = [
    { path: '', label: 'Builder' },
    { path: '/versions', label: 'Versions' },
    { path: '/submissions', label: 'Submissions' },
    { path: '/summary', label: 'Summary' },
    { path: '/notifications', label: 'Notifications' },
    { path: '/recipe', label: 'CRM recipe' },
  ];
  const items = $derived(
    tabs.map((tab) => ({
      href: root + tab.path,
      label: tab.label,
      active: tab.path
        ? page.url.pathname.startsWith(root + tab.path)
        : page.url.pathname === root,
    })),
  );
</script>

<SectionNav label="Form sections" {items} />

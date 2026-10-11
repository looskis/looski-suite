<script lang="ts">
  import { page } from '$app/state';
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
</script>

<nav class="form-tabs" aria-label="Form sections">
  {#each tabs as tab}
    {@const active = tab.path
      ? page.url.pathname.startsWith(root + tab.path)
      : page.url.pathname === root}
    <a href={root + tab.path} aria-current={active ? 'page' : undefined}
      >{tab.label}</a
    >
  {/each}
</nav>

<style>
  .form-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 16px;
    margin-bottom: 20px;
    font-size: 13px;
  }
  .form-tabs a {
    text-decoration: none;
    color: var(--muted);
    padding: 6px 0;
  }
  .form-tabs a[aria-current='page'] {
    color: var(--accent);
    font-weight: 650;
    border-bottom: 2px solid var(--accent);
  }
</style>

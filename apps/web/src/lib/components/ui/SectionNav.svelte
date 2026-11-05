<script lang="ts">
  let {
    label,
    items,
  }: {
    label: string;
    items: readonly { href: string; label: string; active: boolean }[];
  } = $props();
</script>

<nav class="section-nav" aria-label={label}>
  {#each items as item (item.href)}
    <a href={item.href} aria-current={item.active ? 'page' : undefined}>
      {item.label}
    </a>
  {/each}
</nav>

<style>
  .section-nav {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-4);
    margin-bottom: var(--space-5);
    font-size: var(--font-size-13);
  }
  .section-nav a {
    text-decoration: none;
    color: var(--text-secondary);
    padding: var(--space-1-5) 0;
  }
  .section-nav a[aria-current='page'] {
    color: var(--accent);
    font-weight: var(--font-weight-heading);
    border-bottom: 2px solid var(--accent);
  }
</style>

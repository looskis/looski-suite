<script lang="ts">
  import { brand } from '$lib/brand';
  let { compact = false }: { compact?: boolean } = $props();
</script>

<footer class:compact>
  <a class="made-by" href={brand.website}
    ><img src="/brand/looski-mark.svg" alt="" width="22" height="22" />Made by
    Looski<span aria-hidden="true">↗</span></a
  >
  <p>Open source. Yours to make your own.</p>
  <nav aria-label="Project links">
    <a href={brand.website}>Discover Looski</a>
    <a href={brand.source}>GitHub</a>
    {#if !compact}<a href={brand.contribute}>Contribute</a>{/if}
    <a href={brand.license}>AGPLv3</a>
  </nav>
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 18px 32px;
    padding: 28px var(--brand-gutter);
    border-top: 1px solid var(--brand-line);
    width: min(1280px, 100%);
    margin-inline: auto;
    font-size: 12px;
  }
  .made-by {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--brand-ink);
    text-decoration: none;
    font-weight: 500;
  }
  p {
    margin: 0;
    color: var(--brand-muted);
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 20px;
    margin-left: auto;
  }
  nav a {
    color: var(--brand-muted);
    text-decoration: none;
  }
  a:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .compact p {
    display: none;
  }
  @media (max-width: 620px) {
    footer {
      gap: 16px;
    }
    footer p {
      display: none;
    }
    nav {
      width: 100%;
      margin-left: 0;
      gap: 18px;
    }
  }
</style>

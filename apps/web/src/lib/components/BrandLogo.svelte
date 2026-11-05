<script lang="ts">
  let { href = '/', inverse = false }: { href?: string; inverse?: boolean } =
    $props();
</script>

<a class="brand-logo" class:inverse {href} aria-label="Looski Suite home">
  <img src="/brand/looski-mark.svg" alt="" width="36" height="36" />
  <span>looski<span class="suite-label">suite</span></span>
</a>

<style>
  .brand-logo {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--brand-ink);
    text-decoration: none;
    flex-shrink: 0;
  }
  .brand-logo > span {
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.07em;
  }
  .suite-label {
    margin-left: 8px;
    font: 400 10px var(--brand-mono);
    letter-spacing: 0.03em;
    color: var(--brand-muted);
  }
  .inverse {
    color: var(--brand-night-text);
  }
  .inverse .suite-label {
    color: var(--brand-night-muted);
  }
  img {
    display: block;
  }
</style>

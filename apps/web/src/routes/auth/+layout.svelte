<script lang="ts">
  let { children } = $props();
</script>

{@render children()}

<style>
  :global(body) {
    margin: 0;
    background: #f4f3ee;
    color: #183e35;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  :global(main) {
    max-width: 480px;
    margin: 64px auto;
    padding: 24px;
  }
  :global(a) {
    color: #183e35;
  }
  :global(p) {
    line-height: 1.6;
  }
  :global(label) {
    display: block;
    margin: 24px 0;
  }
  :global(input) {
    box-sizing: border-box;
    width: 100%;
    margin-top: 8px;
    padding: 13px;
    border: 1px solid #aabcae;
    border-radius: 8px;
    font: inherit;
  }
  :global(button) {
    display: block;
    width: 100%;
    margin: 16px 0;
    padding: 14px;
    border: 0;
    border-radius: 8px;
    color: white;
    background: #183e35;
    font: inherit;
    cursor: pointer;
  }
  :global(:focus-visible) {
    outline: 3px solid #76a990;
    outline-offset: 3px;
  }
</style>

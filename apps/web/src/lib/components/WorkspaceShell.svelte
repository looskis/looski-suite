<script lang="ts">
  let { children } = $props();
</script>

<div class="workspace-app">{@render children()}</div>

<style>
  :global(body) {
    margin: 0;
    background: #f4f3ee;
    color: #183e35;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  :global(.workspace-app) {
    max-width: 1120px;
    margin: auto;
    padding: 32px 24px 80px;
  }
  :global(.workspace-app header),
  :global(.workspace-app .toolbar) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
    margin-bottom: 24px;
    flex-wrap: wrap;
  }
  :global(.workspace-app a) {
    color: #285b4b;
  }
  :global(.workspace-app h1) {
    font-size: clamp(32px, 5vw, 48px);
    font-weight: 500;
    letter-spacing: -0.04em;
    margin: 30px 0 12px;
  }
  :global(.workspace-app h2) {
    font-size: 22px;
    font-weight: 550;
  }
  :global(.workspace-app p) {
    line-height: 1.6;
  }
  :global(.workspace-app .muted) {
    color: #526b61;
  }
  :global(.workspace-app section) {
    background: #fafbf6;
    border: 1px solid #cdd8cb;
    padding: 24px;
    border-radius: 14px;
    margin-top: 24px;
  }
  :global(.workspace-app label) {
    display: block;
    font-size: 14px;
    margin: 16px 0;
  }
  :global(.workspace-app input:not([type='checkbox'])),
  :global(.workspace-app textarea),
  :global(.workspace-app select) {
    box-sizing: border-box;
    display: block;
    width: 100%;
    padding: 12px;
    border: 1px solid #aabcae;
    border-radius: 7px;
    margin-top: 8px;
    font: inherit;
    background: white;
    color: #183e35;
  }
  :global(.workspace-app button),
  :global(.workspace-app .button) {
    border: 1px solid #285b4b;
    border-radius: 7px;
    padding: 11px 17px;
    background: #285b4b;
    color: white;
    cursor: pointer;
    font: inherit;
    text-decoration: none;
  }
  :global(.workspace-app button.secondary) {
    color: #285b4b;
    background: transparent;
  }
  :global(.workspace-app button:focus-visible),
  :global(.workspace-app a:focus-visible),
  :global(.workspace-app input:focus-visible),
  :global(.workspace-app textarea:focus-visible) {
    outline: 3px solid #d99432;
    outline-offset: 3px;
  }
  :global(.workspace-app .columns) {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
  }
  :global(.workspace-app .check) {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    line-height: 1.5;
  }
  :global(.workspace-app .error) {
    padding: 16px;
    border: 1px solid #bb6644;
    border-radius: 8px;
    background: #fff1e9;
    color: #78391f;
  }
  :global(.workspace-app .table-wrap) {
    overflow-x: auto;
  }
  :global(.workspace-app table) {
    border-collapse: collapse;
    width: 100%;
    text-align: left;
  }
  :global(.workspace-app th),
  :global(.workspace-app td) {
    padding: 16px 10px;
    border-bottom: 1px solid #dce3d7;
    overflow-wrap: anywhere;
  }
  :global(.workspace-app th) {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #526b61;
  }
  :global(.workspace-app .inline) {
    display: flex;
    align-items: end;
    gap: 12px;
  }
  @media (max-width: 640px) {
    :global(.workspace-app .columns) {
      grid-template-columns: 1fr;
      gap: 0;
    }
    :global(.workspace-app section) {
      padding: 16px;
    }
  }
</style>

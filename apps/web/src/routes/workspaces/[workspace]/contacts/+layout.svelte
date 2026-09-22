<script lang="ts">
  let { children } = $props();
</script>

<div class="contacts-app">{@render children()}</div>

<style>
  :global(body) {
    margin: 0;
    background: #f4f3ee;
    color: #183e35;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  :global(.contacts-app) {
    max-width: 1120px;
    margin: auto;
    padding: 32px 24px 80px;
  }
  :global(.contacts-app header),
  :global(.contacts-app .toolbar) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
    margin-bottom: 24px;
    flex-wrap: wrap;
  }
  :global(.contacts-app a) {
    color: #285b4b;
  }
  :global(.contacts-app h1) {
    font-size: clamp(32px, 5vw, 48px);
    font-weight: 500;
    letter-spacing: -0.04em;
    margin: 30px 0 12px;
  }
  :global(.contacts-app h2) {
    font-size: 22px;
    font-weight: 550;
  }
  :global(.contacts-app p) {
    line-height: 1.6;
  }
  :global(.contacts-app .muted) {
    color: #526b61;
  }
  :global(.contacts-app section) {
    background: #fafbf6;
    border: 1px solid #cdd8cb;
    padding: 24px;
    border-radius: 14px;
    margin-top: 24px;
  }
  :global(.contacts-app label) {
    display: block;
    font-size: 14px;
    margin: 16px 0;
  }
  :global(.contacts-app input:not([type='checkbox'])),
  :global(.contacts-app textarea) {
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
  :global(.contacts-app button),
  :global(.contacts-app .button) {
    border: 1px solid #285b4b;
    border-radius: 7px;
    padding: 11px 17px;
    background: #285b4b;
    color: white;
    cursor: pointer;
    font: inherit;
    text-decoration: none;
  }
  :global(.contacts-app button.secondary) {
    color: #285b4b;
    background: transparent;
  }
  :global(.contacts-app button:focus-visible),
  :global(.contacts-app a:focus-visible),
  :global(.contacts-app input:focus-visible),
  :global(.contacts-app textarea:focus-visible) {
    outline: 3px solid #d99432;
    outline-offset: 3px;
  }
  :global(.contacts-app .columns) {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
  }
  :global(.contacts-app .check) {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    line-height: 1.5;
  }
  :global(.contacts-app .error) {
    padding: 16px;
    border: 1px solid #bb6644;
    border-radius: 8px;
    background: #fff1e9;
    color: #78391f;
  }
  :global(.contacts-app .table-wrap) {
    overflow-x: auto;
  }
  :global(.contacts-app table) {
    border-collapse: collapse;
    width: 100%;
    text-align: left;
  }
  :global(.contacts-app th),
  :global(.contacts-app td) {
    padding: 16px 10px;
    border-bottom: 1px solid #dce3d7;
    overflow-wrap: anywhere;
  }
  :global(.contacts-app th) {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #526b61;
  }
  :global(.contacts-app .inline) {
    display: flex;
    align-items: end;
    gap: 12px;
  }
  @media (max-width: 640px) {
    :global(.contacts-app .columns) {
      grid-template-columns: 1fr;
      gap: 0;
    }
    :global(.contacts-app section) {
      padding: 16px;
    }
  }
</style>

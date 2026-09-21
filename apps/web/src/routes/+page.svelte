<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
</script>

<svelte:head>
  <title>Looski Suite</title>
  <meta name="description" content="Your business software, together." />
</svelte:head>

<main>
  <p class="eyebrow">LOOSKI SUITE</p>
  <h1>A place for your<br />business to grow.</h1>
  <p class="intro">
    Your contacts, work, and customer relationships. Together, on your terms.
  </p>
  <section aria-labelledby="setup-title">
    <div class="status">
      <span class:connected={data.system}></span>Development preview
    </div>
    <h2 id="setup-title">Your workspace starts here.</h2>
    <p>
      Sign in to your business workspace. Contacts and the rest of your tools
      are coming next.
    </p>
    <a href="/login">Sign in →</a>
    <p class="connection" role="status">
      {#if data.system}
        Connected to {data.system.name} · v{data.system.version}
      {:else}
        The service is unavailable. Start the local stack and refresh this page.
      {/if}
    </p>
  </section>
  <footer>Built for business owners. Open source under AGPLv3.</footer>
</main>

<style>
  :global(body) {
    margin: 0;
    background: #f4f3ee;
    color: #183e35;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  main {
    max-width: 880px;
    margin: 0 auto;
    padding: 64px 28px;
  }
  .eyebrow {
    font-size: 12px;
    letter-spacing: 0.2em;
    font-weight: 750;
    margin-bottom: 70px;
  }
  h1 {
    font-size: clamp(40px, 7vw, 72px);
    font-weight: 500;
    letter-spacing: -0.055em;
    line-height: 1.08;
    margin: 0 0 24px;
  }
  .intro {
    max-width: 520px;
    font-size: 19px;
    line-height: 1.6;
    color: #526b61;
  }
  section {
    margin-top: 48px;
    border: 1px solid #cdd8cb;
    border-radius: 18px;
    padding: 30px;
    background: #fafbf6;
  }
  .status {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .status span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #b17b37;
  }
  .status .connected {
    background: #368562;
  }
  h2 {
    font-size: 23px;
    font-weight: 550;
  }
  section p {
    line-height: 1.6;
    color: #526b61;
  }
  .connection {
    border-top: 1px solid #dde4d8;
    padding-top: 18px;
    margin-bottom: 0;
    font-size: 13px;
  }
  footer {
    margin-top: 60px;
    font-size: 12px;
    color: #526b61;
  }
</style>

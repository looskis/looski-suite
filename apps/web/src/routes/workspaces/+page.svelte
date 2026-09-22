<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>Your workspaces · Looski Suite</title></svelte:head>
<main>
  <header>
    <a href="/">LOOSKI SUITE</a>
    <form method="POST" action="/logout"><button>Sign out</button></form>
  </header>
  <p class="eyebrow">YOUR BUSINESS, TOGETHER</p>
  <h1>Your workspaces.</h1>
  <p class="intro">A home for your team and the work ahead.</p>
  {#each data.workspaces as workspace (workspace.id)}
    <article>
      <span class="icon" aria-hidden="true">⌂</span>
      <div>
        <h2>{workspace.name}</h2>
        <p>Keep your business relationships in one place.</p>
      </div>
      <a class="badge" href={`/workspaces/${workspace.id}/contacts`}
        >Open Contacts →</a
      ><a href={`/workspaces/${workspace.id}/tasks`}>Tasks →</a><a
        href={`/workspaces/${workspace.id}/jobs`}>Background work →</a
      >
    </article>
  {:else}
    <article>
      <div>
        <h2>No workspaces available</h2>
        <p>
          Your login is valid, but you do not currently have access to a
          workspace. Contact your installation owner.
        </p>
      </div>
    </article>
  {/each}
</main>

<style>
  :global(body) {
    margin: 0;
    background: #f4f3ee;
    color: #183e35;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  main {
    max-width: 960px;
    margin: auto;
    padding: 40px 24px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 90px;
  }
  header a {
    color: inherit;
    text-decoration: none;
    font-size: 12px;
    letter-spacing: 0.2em;
    font-weight: 750;
  }
  button {
    background: transparent;
    color: #183e35;
    border: 1px solid #aabcae;
    padding: 10px 16px;
    border-radius: 8px;
    cursor: pointer;
  }
  .eyebrow {
    font-size: 11px;
    letter-spacing: 0.15em;
    color: #526b61;
  }
  h1 {
    font-size: clamp(38px, 6vw, 60px);
    letter-spacing: -0.05em;
    font-weight: 500;
    margin: 18px 0;
  }
  .intro {
    color: #526b61;
    font-size: 18px;
    margin-bottom: 40px;
  }
  article {
    display: flex;
    align-items: center;
    gap: 22px;
    background: #fafbf6;
    border: 1px solid #cdd8cb;
    border-radius: 14px;
    padding: 28px;
    margin-bottom: 16px;
  }
  .icon {
    font-size: 32px;
  }
  h2 {
    font-size: 22px;
    margin: 0;
    font-weight: 550;
    overflow-wrap: anywhere;
  }
  article p {
    color: #526b61;
    line-height: 1.6;
    margin-bottom: 0;
  }
  .badge {
    margin-left: auto;
    border-radius: 20px;
    background: #e2edde;
    padding: 6px 12px;
    font-size: 12px;
  }
  @media (max-width: 500px) {
    article {
      align-items: flex-start;
      padding: 20px;
    }
    .icon,
    .badge {
      display: none;
    }
  }
</style>

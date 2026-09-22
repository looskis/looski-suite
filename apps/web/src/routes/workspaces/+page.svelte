<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>Your workspaces · Looski Suite</title></svelte:head>
<main>
  <header>
    <a class="brand" href="/"><span class="brand-mark">l</span>looski</a>
    <form method="POST" action="/logout">
      <button class="secondary">Sign out</button>
    </form>
  </header>
  <p class="eyebrow">YOUR BUSINESS, TOGETHER</p>
  <h1>Your workspaces</h1>
  {#if data.pendingInvitation}<p>
      <a href="/invitations/accept"
        >Review your pending workspace invitation →</a
      >
    </p>{/if}
  <p class="intro">A home for your team and the work ahead.</p>
  {#each data.workspaces as workspace (workspace.id)}
    <article>
      <span class="icon" aria-hidden="true"
        >{workspace.name.slice(0, 1).toUpperCase()}</span
      >
      <div>
        <h2>{workspace.name}</h2>
        <p>Keep your business relationships in one place.</p>
      </div>
      <div class="workspace-actions">
        <a class="badge" href={`/workspaces/${workspace.id}/contacts`}
          >Open Contacts →</a
        ><a href={`/workspaces/${workspace.id}/tasks`}>Tasks →</a><a
          href={`/workspaces/${workspace.id}/jobs`}>Background work →</a
        ><a href={`/workspaces/${workspace.id}/exports`}>Export data →</a><a
          href={`/workspaces/${workspace.id}/files`}>Files →</a
        >
      </div>
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
  main {
    max-width: 1080px;
    padding: 0 40px 80px;
    margin: auto;
  }
  header {
    min-height: 96px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--line);
    margin-bottom: 56px;
  }
  h1 {
    font-size: 38px;
    margin: 12px 0;
  }
  .intro {
    color: var(--muted);
    font-size: 15px;
    margin-bottom: 32px;
  }
  article {
    display: grid;
    grid-template-columns: 48px 1fr;
    gap: 16px 20px;
    background: white;
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 26px;
    margin: 18px 0;
    box-shadow: var(--shadow);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border: 1px solid #dae8df;
    border-radius: 10px;
    background: var(--tint);
    color: var(--accent);
    font-size: 20px;
    font-weight: 600;
  }
  h2 {
    margin: 2px 0 6px;
    overflow-wrap: anywhere;
  }
  article p {
    color: var(--muted);
    font-size: 13px;
    margin: 0;
  }
  .workspace-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px 24px;
    grid-column: 2;
    padding-top: 18px;
    border-top: 1px solid var(--line);
  }
  .workspace-actions a {
    font-size: 12px;
    text-decoration: none;
  }
  .workspace-actions a:hover {
    text-decoration: underline;
  }
  .badge {
    background: var(--accent);
    color: white;
    padding: 10px 14px;
    border-radius: 7px;
    font-weight: 600;
  }
  @media (max-width: 600px) {
    main {
      padding: 0 20px 60px;
    }
    header {
      min-height: 80px;
      margin-bottom: 36px;
    }
    h1 {
      font-size: 30px;
    }
    article {
      padding: 20px;
      grid-template-columns: 36px 1fr;
      gap: 14px;
    }
    .icon {
      width: 36px;
      height: 36px;
      font-size: 16px;
    }
    .workspace-actions {
      grid-column: 1 / -1;
      gap: 16px;
    }
  }
</style>

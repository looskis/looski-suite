<script lang="ts">
  import Icon from './Icon.svelte';
  import AppearanceControls from './ui/AppearanceControls.svelte';
  let {
    email,
    workspaceRoot,
    docked = false,
  }: { email: string; workspaceRoot?: string; docked?: boolean } = $props();
  let menu: HTMLElement;
</script>

<button
  type="button"
  class="account-avatar"
  class:docked
  popovertarget="account-menu"
  aria-label="Open account menu"
  ><span class="avatar-initials">{email.slice(0, 2).toUpperCase()}</span
  >{#if docked}<span class="account-label"
      ><strong>My account</strong><small>{email}</small></span
    ><span aria-hidden="true">⌃</span>{/if}</button
>
<div
  id="account-menu"
  bind:this={menu}
  popover="auto"
  class="account-menu"
  class:docked
>
  <div class="account-identity">
    <strong>My account</strong><span>{email}</span>
  </div>
  <AppearanceControls />
  <nav aria-label="Account">
    {#if workspaceRoot}
      <a
        href={`${workspaceRoot}/settings#profile`}
        onclick={() => menu?.hidePopover()}>Profile <span>↗</span></a
      ><a href={`${workspaceRoot}/settings`} onclick={() => menu?.hidePopover()}
        >Settings <Icon name="grid" size={15} /></a
      >{/if}
    <a href="/workspaces" onclick={() => menu?.hidePopover()}
      >Switch workspace <span>⇄</span></a
    >
  </nav>
  <form method="POST" action="/logout">
    <button class="logout"><Icon name="logout" size={16} />Sign out</button>
  </form>
</div>

<style>
  .account-avatar {
    position: fixed;
    z-index: 10;
    bottom: 20px;
    right: 22px;
    width: 42px;
    height: 42px;
    padding: 0;
    border-radius: 50%;
    background: var(--text-primary);
    color: var(--surface-panel);
    border: 3px solid var(--surface-panel);
    box-shadow: 0 3px 12px #0003;
    font-size: 0.75rem;
  }
  .account-menu {
    position: fixed;
    inset: auto 22px 72px auto;
    margin: 0;
    width: min(300px, calc(100vw - 32px));
    border: 1px solid var(--border-default);
    border-radius: 12px;
    background: var(--surface-panel);
    color: var(--text-primary);
    padding: 18px;
    box-shadow: 0 16px 60px #0003;
  }
  .account-identity {
    display: grid;
    gap: 4px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-default);
  }
  .account-identity strong {
    font-size: 0.8125rem;
  }
  .account-identity span {
    color: var(--text-secondary);
    font-size: 0.75rem;
    overflow-wrap: anywhere;
  }
  .account-menu nav {
    padding: 8px 0;
    border-top: 1px solid var(--border-default);
  }
  .account-menu nav a {
    display: flex;
    justify-content: space-between;
    padding: 9px 7px;
    text-decoration: none;
    color: var(--text-primary);
    font-size: 0.75rem;
    border-radius: 5px;
  }
  .account-menu nav a:hover {
    background: var(--surface-hover);
  }
  .account-menu form {
    border-top: 1px solid var(--border-default);
    padding-top: 7px;
  }
  .logout {
    display: flex;
    gap: 9px;
    align-items: center;
    width: 100%;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.75rem;
    padding: 8px 7px;
  }

  .account-avatar.docked {
    position: static;
    display: flex;
    justify-content: flex-start;
    gap: 10px;
    width: 100%;
    height: auto;
    min-height: 52px;
    margin-top: 12px;
    padding: 10px 8px;
    background: transparent;
    color: var(--text-primary);
    border: 0;
    border-top: 1px solid var(--border-default);
    border-radius: 0;
    box-shadow: none;
  }
  .docked .avatar-initials {
    display: grid;
    place-items: center;
    flex: 0 0 30px;
    height: 30px;
    border-radius: 50%;
    background: var(--text-primary);
    color: var(--surface-panel);
    font-size: 0.625rem;
  }
  .account-label {
    flex: 1;
    min-width: 0;
    text-align: left;
    display: grid;
    gap: 3px;
  }
  .account-label strong {
    font-size: 0.75rem;
    font-weight: 550;
  }
  .account-label small {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 0.625rem;
    font-weight: 400;
  }
  .account-menu.docked {
    inset: auto auto 80px 12px;
  }
</style>

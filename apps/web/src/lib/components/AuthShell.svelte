<script lang="ts">
  let { children } = $props();
</script>

<div class="auth-frame">
  <header>
    <a class="brand" href="/"
      ><span class="brand-mark">l</span>looski<span>suite</span></a
    ><a class="back" href="/">← Back to home</a>
  </header>
  <div class="auth-content">
    <aside>
      <p class="eyebrow">YOUR BUSINESS, TOGETHER</p>
      <h2>A place for<br />good work.</h2>
      <p>
        Your people, your plans, your next step.<br />Pick up where you left
        off.
      </p>
      <div class="auth-art" aria-hidden="true">
        <span></span><span></span><span></span>
      </div>
      <small>Independent tools. One shared home.</small>
    </aside>
    <div class="auth-card">{@render children()}</div>
  </div>
  <footer>Looski Suite · Business software, on your terms.</footer>
</div>

<style>
  .auth-frame {
    max-width: 1160px;
    padding: 0 40px;
    margin: auto;
  }
  header {
    min-height: 96px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--line);
  }
  .brand > span:last-child {
    font-size: 12px;
    font-weight: 400;
    color: var(--muted);
    letter-spacing: 0;
  }
  .back {
    font-size: 12px;
    color: var(--muted);
    text-decoration: none;
  }
  .auth-content {
    min-height: calc(100dvh - 170px);
    display: grid;
    grid-template-columns: 1fr 1fr;
    align-items: center;
    gap: 72px;
    padding: 56px 30px;
  }
  aside h2 {
    font-size: clamp(38px, 4.5vw, 58px);
    font-weight: 550;
    letter-spacing: -0.05em;
    line-height: 1.12;
    margin: 24px 0;
  }
  aside > p:not(.eyebrow) {
    font-size: 15px;
    color: var(--muted);
  }
  aside small {
    font-size: 11px;
  }
  .auth-art {
    display: flex;
    align-items: center;
    padding: 30px 0;
  }
  .auth-art span {
    width: 64px;
    height: 64px;
    background: #dbe9e0;
    border: 5px solid var(--canvas);
    border-radius: 18px;
    transform: rotate(-10deg);
  }
  .auth-art span + span {
    margin-left: -14px;
    background: #e8e6d9;
    transform: rotate(5deg);
  }
  .auth-art span:last-child {
    background: #dce5ed;
    transform: rotate(18deg);
  }
  .auth-card {
    background: white;
    padding: 36px;
    border: 1px solid var(--line);
    border-radius: 16px;
    box-shadow: var(--shadow);
    min-width: 0;
  }
  :global(.auth-card main) {
    margin: 0;
    padding: 0;
    max-width: none;
  }
  :global(.auth-card h1) {
    font-size: 27px;
    margin: 10px 0;
  }
  :global(.auth-card p) {
    font-size: 13px;
    color: var(--muted);
  }
  :global(.auth-card .eyebrow) {
    color: var(--accent);
    font-size: 10px;
  }
  :global(.auth-card label) {
    margin-bottom: 7px;
  }
  :global(.auth-card button) {
    width: 100%;
    margin-top: 22px;
    justify-content: space-between;
  }
  :global(.auth-card .help) {
    border-top: 1px solid var(--line);
    padding-top: 20px;
    margin-top: 24px;
    font-size: 11px;
  }
  :global(.auth-card a) {
    font-size: 12px;
  }
  :global(.auth-card .error) {
    color: #953b31;
  }
  footer {
    min-height: 74px;
    padding: 24px 0;
    font-size: 11px;
    color: var(--muted);
    border-top: 1px solid var(--line);
  }
  @media (max-width: 800px) {
    .auth-content {
      padding-inline: 0;
      gap: 30px;
    }
    .auth-card {
      padding: 28px;
    }
  }
  @media (max-width: 620px) {
    .auth-frame {
      padding: 0 22px;
    }
    header {
      min-height: 80px;
    }
    aside {
      display: none;
    }
    .auth-content {
      display: block;
      min-height: auto;
      padding: 40px 0;
    }
    .auth-card {
      padding: 26px 22px;
    }
  }
</style>

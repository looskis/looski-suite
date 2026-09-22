<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
</script>

<svelte:head>
  <title>Looski Suite · Your business, together</title>
  <meta
    name="description"
    content="A shared home for your contacts, customer relationships, and everyday work."
  />
</svelte:head>

<div class="welcome">
  <header class="welcome-nav">
    <a class="brand" href="/"
      ><span class="brand-mark">l</span>looski<span class="suite-label"
        >suite</span
      ></a
    >
    <a class="button secondary" href="/login"
      >Sign in <Icon name="arrow" size={16} /></a
    >
  </header>
  <main id="main-content">
    <section class="hero">
      <div class="hero-copy">
        <p class="eyebrow"><span></span>A little less scattered.</p>
        <h1>Your business.<br /><span>All together.</span></h1>
        <p class="intro">
          The people, the possibilities, and the work ahead. Give it all a place
          to call home.
        </p>
        <a class="button hero-cta" href="/workspaces"
          >Open your workspace <Icon name="arrow" size={18} /></a
        >
        <p class="hero-note">
          Built for business owners. Made to work your way.
        </p>
      </div>
      <div class="overview" aria-label="Connected tools for your business">
        <div class="overview-heading">
          <span class="eyebrow">A connected workspace</span><span
            class="overview-mark"
            aria-hidden="true">✳</span
          >
        </div>
        <div class="workspace-preview">
          <div class="preview-top">
            <span class="brand-mark">l</span><span
              >Your business<small>Everything has a place.</small></span
            ><span class="preview-dot"></span>
          </div>
          <div class="journey-card">
            <span class="tile-icon"><Icon name="people" size={21} /></span>
            <div>
              <strong>Start with a relationship</strong>
              <p>People & organizations</p>
            </div>
            <span class="step">01</span>
          </div>
          <div class="connector" aria-hidden="true"></div>
          <div class="journey-card">
            <span class="tile-icon amber"><Icon name="grid" size={21} /></span>
            <div>
              <strong>See what’s possible</strong>
              <p>Opportunities & deals</p>
            </div>
            <span class="step">02</span>
          </div>
          <div class="connector" aria-hidden="true"></div>
          <div class="journey-card">
            <span class="tile-icon blue"><Icon name="tasks" size={21} /></span>
            <div>
              <strong>Move the work forward</strong>
              <p>Tasks, activities & files</p>
            </div>
            <span class="step">03</span>
          </div>
        </div>
        <p class="overview-note">Independent tools. One shared foundation.</p>
      </div>
    </section>
    <section class="features" aria-label="Your workspace tools">
      <article>
        <span class="feature-number">01 / RELATIONSHIPS</span>
        <h2>Remember the people.</h2>
        <p>
          Contacts, organizations, and the context that makes every conversation
          count.
        </p>
      </article>
      <article>
        <span class="feature-number">02 / POSSIBILITIES</span>
        <h2>Make it your process.</h2>
        <p>
          Opportunities and deals with stages, statuses, and workflows that fit
          your business.
        </p>
      </article>
      <article>
        <span class="feature-number">03 / EVERYDAY WORK</span>
        <h2>Keep things moving.</h2>
        <p>
          Shared tasks, files, and follow-ups that keep the next step close at
          hand.
        </p>
      </article>
    </section>
  </main>
  <footer>
    <span>Business software, on your terms.</span><span
      class="connection"
      role="status"
      ><i class:connected={data.system}></i>{#if data.system}Connected to {data
          .system.name}{:else}Service unavailable{/if}</span
    ><span>Open source · AGPLv3</span>
  </footer>
</div>

<style>
  .welcome {
    max-width: 1240px;
    margin: auto;
    padding: 0 48px;
  }
  .welcome-nav {
    min-height: 100px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--line);
  }
  .brand {
    font-size: 25px;
  }
  .suite-label {
    font-weight: 400;
    font-size: 13px;
    color: var(--muted);
    margin-left: -4px;
    align-self: end;
    margin-bottom: 4px;
    letter-spacing: 0;
  }
  .hero {
    display: grid;
    grid-template-columns: 1.15fr 1fr;
    align-items: center;
    gap: 64px;
    padding: 84px 0 76px;
  }
  .eyebrow {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 26px;
  }
  .eyebrow span {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
  }
  h1 {
    font-size: clamp(44px, 5.4vw, 72px);
    letter-spacing: -0.055em;
    font-weight: 600;
    line-height: 1.06;
    margin: 0 0 28px;
  }
  h1 span {
    color: var(--accent);
  }
  .intro {
    font-size: 18px;
    line-height: 1.75;
    color: var(--muted);
    max-width: 400px;
    margin: 0 0 32px;
  }
  .hero-cta {
    padding: 14px 22px;
    min-height: 50px;
    gap: 24px;
  }
  .hero-note {
    font-size: 11px;
    color: var(--muted);
    margin-top: 17px;
  }
  .overview {
    background: #e8eeea;
    border: 1px solid #dae4dc;
    border-radius: 20px;
    padding: 28px;
    position: relative;
  }
  .overview-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 22px;
  }
  .overview-heading .eyebrow {
    font-size: 9px;
    margin: 0;
    color: #476457;
  }
  .overview-mark {
    color: #638e72;
    font-size: 28px;
    line-height: 1;
  }
  .workspace-preview {
    background: #ffffffb3;
    border: 1px solid #fff;
    border-radius: 13px;
    padding: 20px;
    box-shadow: 0 16px 40px #1d412110;
  }
  .preview-top {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-bottom: 20px;
    margin-bottom: 20px;
    border-bottom: 1px solid var(--line);
    font-size: 13px;
    font-weight: 650;
  }
  .preview-top .brand-mark {
    width: 28px;
    height: 28px;
    border-radius: 7px;
    font-size: 20px;
  }
  .preview-top small {
    display: block;
    margin-top: 4px;
    font-size: 10px;
    font-weight: 400;
  }
  .preview-dot {
    margin-left: auto;
    width: 6px;
    height: 6px;
    background: #6fa186;
    border-radius: 50%;
  }
  .journey-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 15px 12px;
    background: white;
    border: 1px solid #e5ebe7;
    border-radius: 9px;
    box-shadow: 0 3px 5px #18372803;
  }
  .journey-card strong {
    font-size: 12px;
    font-weight: 600;
  }
  .journey-card p {
    font-size: 10px;
    color: var(--muted);
    margin: 4px 0 0;
  }
  .tile-icon {
    display: grid;
    place-items: center;
    color: #376d54;
    background: #edf5ef;
    border-radius: 8px;
    width: 38px;
    height: 38px;
    flex: 0 0 auto;
  }
  .tile-icon.amber {
    color: #967344;
    background: #faf3e8;
  }
  .tile-icon.blue {
    color: #4c6f91;
    background: #eef3f9;
  }
  .step {
    font-size: 9px;
    color: #95a19c;
    margin-left: auto;
  }
  .connector {
    height: 12px;
    margin-left: 31px;
    border-left: 1px dashed #b2c9b9;
  }
  .overview-note {
    font-size: 10px;
    color: #647e6d;
    text-align: center;
    margin: 18px 0 0;
  }
  .features {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    padding: 34px 0;
    gap: 36px;
  }
  .features article + article {
    border-left: 1px solid var(--line);
    padding-left: 36px;
  }
  .feature-number {
    font-size: 9px;
    letter-spacing: 0.12em;
    font-weight: 600;
    color: var(--muted);
  }
  .features h2 {
    margin: 14px 0 9px;
    font-size: 17px;
  }
  .features p {
    color: var(--muted);
    font-size: 13px;
    margin: 0;
    max-width: 270px;
  }
  footer {
    padding: 28px 0;
    display: flex;
    justify-content: space-between;
    gap: 20px;
    color: var(--muted);
    font-size: 10px;
  }
  .connection {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .connection i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: #ac7939;
  }
  .connection i.connected {
    background: #558769;
  }
  @media (max-width: 960px) {
    .welcome {
      padding: 0 28px;
    }
    .hero {
      gap: 28px;
    }
    .overview {
      padding: 20px;
    }
    .workspace-preview {
      padding: 14px;
    }
  }
  @media (max-width: 720px) {
    .welcome {
      padding: 0 22px;
    }
    .welcome-nav {
      min-height: 80px;
    }
    .hero {
      grid-template-columns: 1fr;
      padding: 48px 0;
      gap: 36px;
    }
    .hero-copy {
      max-width: 500px;
    }
    .intro {
      font-size: 16px;
    }
    .overview {
      max-width: 480px;
      width: 100%;
    }
    .features {
      grid-template-columns: 1fr;
      gap: 26px;
    }
    .features article + article {
      padding: 26px 0 0;
      border-left: 0;
      border-top: 1px solid var(--line);
    }
    .features p {
      max-width: 100%;
    }
    footer {
      flex-wrap: wrap;
      line-height: 1.5;
    }
  }
</style>

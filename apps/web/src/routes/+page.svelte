<script lang="ts">
  import BrandLogo from '$lib/components/BrandLogo.svelte';
  import BrandFooter from '$lib/components/BrandFooter.svelte';
  import SuitePreview from '$lib/components/SuitePreview.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { brand } from '$lib/brand';
  const features = [
    {
      icon: 'people',
      title: 'Build better relationships.',
      description:
        'Keep your contacts, organizations, opportunities, and deals connected. Pick up every conversation with context.',
      tools: 'Contacts · Organizations · CRM',
    },
    {
      icon: 'board',
      title: 'Turn plans into progress.',
      description:
        'Give projects a home, tasks an owner, and your team a clear next step. Track the time that goes into the work.',
      tools: 'Projects · Tasks · Time tracking',
    },
    {
      icon: 'form',
      title: 'Make a great first impression.',
      description:
        'Collect the right details with forms, take bookings, and bring new requests into your workflow.',
      tools: 'Forms · Scheduling · Intake',
    },
    {
      icon: 'inbox',
      title: 'Keep the conversation going.',
      description:
        'Give customer requests a shared inbox. Connect replies, internal notes, and the people behind them.',
      tools: 'Helpdesk · Communications',
    },
    {
      icon: 'book',
      title: 'Put knowledge to work.',
      description:
        'Publish useful content, document how things get done, and keep your team’s files within reach.',
      tools: 'Content · Knowledge base · Files',
    },
    {
      icon: 'grid',
      title: 'Make it your kind of business.',
      description:
        'Configure your stages and workflows, connect through the API, or bring your own changes to the source.',
      tools: 'Workflows · API · CLI · MCP',
    },
  ] as const;
</script>

<svelte:head>
  <title>Looski Suite — Your whole business. On your terms.</title>
  <meta
    name="description"
    content="Open-source CRM, projects, forms, scheduling, helpdesk, and more. Self-host Looski Suite, customize your workflows, and bring your business tools together without per-seat software fees."
  />
  <meta
    property="og:title"
    content="Looski Suite — Your whole business. On your terms."
  />
  <meta
    property="og:description"
    content="Connected business tools. Open source. Self-hosted. Yours to make your own."
  />
  <meta property="og:type" content="website" />
  <meta name="theme-color" content="#080b0e" />
</svelte:head>

<div class="brand-surface">
  <a class="brand-skip" href="#main-content">Skip to content</a>
  <div class="dark-stage">
    <header class="brand-container site-header">
      <BrandLogo inverse />
      <nav aria-label="Main navigation">
        <a href="#tools">The suite</a><a href="#ownership">Why open source</a><a
          href={brand.website}>Meet Looski <span aria-hidden="true">↗</span></a
        >
      </nav>
      <a class="sign-in" href="/login"
        >Sign in <Icon name="arrow" size={16} /></a
      >
    </header>
  </div>
  <main id="main-content">
    <div class="dark-stage">
      <section class="hero brand-container" aria-labelledby="hero-title">
        <div class="hero-heading">
          <p class="brand-kicker">
            <span></span>Open source. Open possibilities.
          </p>
          <h1 id="hero-title">
            Your whole business.<br /><em>On your terms.</em>
          </h1>
        </div>
        <div class="hero-bottom">
          <p>
            Customers, projects, and everything in between.<br
              class="desktop-break"
            /> Connected tools you can host, customize, and call your own.
          </p>
          <div class="hero-actions">
            <a class="brand-pill mint" href={brand.install}
              >Get Looski Suite <Icon name="arrow" size={17} /></a
            ><a class="brand-link" href="#tools"
              >Explore the tools <span aria-hidden="true">↓</span></a
            >
          </div>
        </div>
        <ul class="hero-proof">
          <li>No per-seat software fees</li>
          <li>Your infrastructure. Your data.</li>
          <li>Built in the open</li>
        </ul>
        <SuitePreview />
        <div class="preview-caption">
          <span>A little less scattered. A lot more connected.</span><a
            href={brand.source}
            >Explore the development preview <span aria-hidden="true">↗</span
            ></a
          >
        </div>
      </section>
    </div>

    <section
      id="tools"
      class="tools brand-container"
      aria-labelledby="tools-title"
    >
      <div class="section-heading">
        <div>
          <p class="brand-kicker">Room for the whole business</p>
          <h2 id="tools-title">
            A lot of tools.<br /><em>One place to belong.</em>
          </h2>
        </div>
        <p>
          From the first hello to the finished project.<br />Start with what you
          need. Grow into the rest.
        </p>
      </div>
      <div class="feature-grid">
        {#each features as feature, i}<article>
            <div class="feature-top">
              <Icon name={feature.icon} size={24} /><span class="brand-kicker"
                >0{i + 1}</span
              >
            </div>
            <h3>{feature.title}</h3>
            <p>{feature.description}</p>
            <span class="tool-list">{feature.tools}</span>
          </article>{/each}
      </div>
    </section>

    <section id="ownership" class="ownership" aria-labelledby="ownership-title">
      <div class="brand-container ownership-grid">
        <div>
          <p class="brand-kicker">Own more. Owe less.</p>
          <h2 id="ownership-title">
            More room to grow.<br /><em>Fewer subscriptions.</em>
          </h2>
          <p class="ownership-intro">
            Your software budget should leave room for your business. Looski
            brings the essentials together without charging you for every person
            who joins.
          </p>
          <a class="brand-link" href={brand.install}
            >Make yourself at home <Icon name="arrow" size={17} /></a
          >
        </div>
        <div class="ownership-points">
          <article>
            <span>01</span>
            <div>
              <h3>Open source, all the way down to you.</h3>
              <p>
                Read the code. Run it yourself. Adapt it to your business. The
                source is available under AGPLv3.
              </p>
            </div>
          </article>
          <article>
            <span>02</span>
            <div>
              <h3>Your team grows. The seat fee doesn’t.</h3>
              <p>
                No per-user software license or separate subscription for each
                Suite tool. You cover your hosting and any services you choose.
              </p>
            </div>
          </article>
          <article>
            <span>03</span>
            <div>
              <h3>Your process gets the final say.</h3>
              <p>
                Make the workflows fit your team, connect your other tools, and
                build on a foundation you can change.
              </p>
            </div>
          </article>
        </div>
      </div>
    </section>

    <section
      class="community brand-container"
      aria-labelledby="community-title"
    >
      <div class="community-copy">
        <p class="brand-kicker">Made by Looski. Built with you.</p>
        <h2 id="community-title">
          Good software gets<br /><em>better together.</em>
        </h2>
        <p>
          Have an idea? Found a rough edge? Want to build something? Help shape
          the tools that help businesses like yours.
        </p>
        <a class="brand-link" href={brand.contribute}
          >Join in on GitHub <Icon name="arrow" size={17} /></a
        >
      </div>
      <div class="source-card">
        <img src="/brand/looski-mark.svg" alt="" width="72" height="72" /><span
          class="brand-kicker">looskis / looski-suite</span
        >
        <h3>Open the source.<br />See what’s possible.</h3>
        <p>
          Explore the code, follow development, and make your first
          contribution.
        </p>
        <a class="brand-pill" href={brand.source}
          >View the project <span aria-hidden="true">↗</span></a
        >
      </div>
    </section>

    <section class="closing brand-container" aria-labelledby="closing-title">
      <div class="closing-inner">
        <div>
          <p class="brand-kicker">Make it yours</p>
          <h2 id="closing-title">
            Your next chapter.<br /><em>Your own workspace.</em>
          </h2>
          <p>
            Get started yourself, or talk to the people behind Looski about
            bringing it to your business.
          </p>
        </div>
        <div class="closing-actions">
          <a class="brand-pill mint" href={brand.install}
            >Start self-hosting <Icon name="arrow" size={17} /></a
          ><a class="brand-link" href={brand.contact}
            >Let’s talk <span aria-hidden="true">↗</span></a
          ><small
            >Already part of a workspace? <a href="/login">Sign in</a></small
          >
        </div>
      </div>
    </section>
  </main>
  <BrandFooter />
</div>

<style>
  .dark-stage {
    background: var(--brand-glow), var(--brand-night);
    color: var(--brand-night-text);
  }
  .site-header {
    min-height: 86px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    border-bottom: 1px solid var(--brand-night-line);
  }
  .site-header nav {
    display: flex;
    align-items: center;
    gap: 30px;
  }
  .site-header a {
    color: var(--brand-night-muted);
    font-size: 12px;
    text-decoration: none;
  }
  .site-header a:hover {
    color: var(--brand-night-text);
  }
  .site-header .sign-in {
    display: inline-flex;
    align-items: center;
    gap: 18px;
    border: 1px solid var(--brand-night-line);
    color: var(--brand-night-text);
    border-radius: 30px;
    padding: 11px 17px;
  }
  .hero {
    padding-top: 65px;
    padding-bottom: 28px;
  }
  .hero-heading .brand-kicker {
    color: var(--brand-mint);
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 24px;
  }
  .hero-heading .brand-kicker span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--brand-spectrum);
  }
  h1 {
    color: var(--brand-night-text);
    font-size: clamp(50px, 6.7vw, 90px);
    line-height: 0.99;
    margin: 0;
    font-weight: 500;
  }
  h1 em {
    color: #c8d5df;
  }
  .hero-bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 36px;
    margin: 30px 0 22px;
  }
  .hero-bottom > p {
    color: var(--brand-night-muted);
    font-size: 16px;
    line-height: 1.7;
    margin: 0;
    max-width: 540px;
  }
  .hero-actions {
    display: flex;
    gap: 22px;
    align-items: center;
    flex-shrink: 0;
  }
  .hero-actions > .brand-link {
    color: var(--brand-night-text);
    font-size: 13px;
  }
  .hero-proof {
    padding: 0;
    margin: 0 0 38px;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 12px 28px;
    font: 9px/1.7 var(--brand-mono);
    color: var(--brand-night-muted);
  }
  .hero-proof li::before {
    content: '✓';
    color: var(--brand-mint);
    margin-right: 8px;
  }
  .preview-caption {
    display: flex;
    justify-content: space-between;
    gap: 20px;
    font-size: 10px;
    color: var(--brand-night-muted);
    margin-top: 18px;
  }
  .preview-caption a {
    color: inherit;
    text-decoration: none;
  }
  .preview-caption a:hover {
    text-decoration: underline;
  }
  .tools {
    padding-top: 96px;
    padding-bottom: 90px;
    scroll-margin-top: 28px;
  }
  .section-heading {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 30px;
    margin-bottom: 45px;
  }
  .brand-kicker {
    color: var(--brand-muted);
    margin: 0 0 18px;
  }
  h2 {
    font-size: clamp(34px, 4vw, 50px);
    line-height: 1.06;
    margin: 0;
  }
  .section-heading > p {
    color: var(--brand-muted);
    font-size: 14px;
    margin: 0 0 4px;
  }
  .feature-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0 32px;
  }
  .feature-grid article {
    padding: 28px 0;
    border-top: 1px solid var(--brand-line);
  }
  .feature-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--brand-muted);
  }
  .feature-top span {
    margin: 0;
    font-size: 9px;
  }
  .feature-grid h3 {
    font-size: 21px;
    line-height: 1.25;
    margin: 20px 0 12px;
  }
  .feature-grid p {
    font-size: 13px;
    color: var(--brand-muted);
    margin: 0;
    min-height: 84px;
  }
  .tool-list {
    display: block;
    font: 9px/1.8 var(--brand-mono);
    color: var(--brand-ink);
    margin-top: 20px;
  }
  .ownership {
    background: #e8ede7;
    border-block: 1px solid var(--brand-line);
  }
  .ownership-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 90px;
    padding-block: 80px;
  }
  .ownership-intro {
    color: var(--brand-muted);
    font-size: 15px;
    margin: 25px 0;
    max-width: 410px;
  }
  .ownership .brand-link {
    color: var(--brand-ink);
  }
  .ownership-points article {
    display: flex;
    gap: 20px;
    padding: 24px 0;
    border-top: 1px solid #cdd7cc;
  }
  .ownership-points article:first-child {
    padding-top: 0;
    border: 0;
  }
  .ownership-points article:last-child {
    padding-bottom: 0;
  }
  .ownership-points article > span {
    font: 10px var(--brand-mono);
    color: var(--brand-muted);
    padding-top: 5px;
  }
  .ownership-points h3 {
    font-size: 19px;
    line-height: 1.4;
    margin: 0 0 9px;
  }
  .ownership-points p {
    color: var(--brand-muted);
    font-size: 13px;
    margin: 0;
  }
  .community {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 90px;
    padding-block: 90px;
    align-items: center;
  }
  .community-copy > p:not(.brand-kicker) {
    max-width: 360px;
    color: var(--brand-muted);
    font-size: 15px;
    margin: 24px 0;
  }
  .source-card {
    padding: 34px 38px;
    border: 1px solid var(--brand-line);
    border-radius: var(--brand-radius);
    background: var(--surface-panel);
    position: relative;
    overflow: hidden;
  }
  .source-card::before {
    content: '';
    position: absolute;
    inset: 0 0 auto;
    height: 2px;
    background: var(--brand-spectrum);
  }
  .source-card img {
    float: right;
    margin: 0 0 20px 18px;
  }
  .source-card .brand-kicker {
    display: block;
    margin-top: 7px;
    font-size: 9px;
  }
  .source-card h3 {
    margin: 25px 0 14px;
    font-size: 29px;
    line-height: 1.1;
  }
  .source-card p {
    font-size: 13px;
    color: var(--brand-muted);
    max-width: 280px;
  }
  .source-card .brand-pill {
    margin-top: 12px;
    font-size: 12px;
    min-height: 42px;
  }
  .closing {
    padding-bottom: 60px;
  }
  .closing-inner {
    display: flex;
    justify-content: space-between;
    gap: 40px;
    align-items: center;
    padding: 55px;
    border-radius: var(--brand-radius);
    background: var(--brand-glow), var(--brand-night);
    color: var(--brand-night-text);
    position: relative;
  }
  .closing-inner h2 {
    color: inherit;
  }
  .closing-inner .brand-kicker {
    color: var(--brand-mint);
  }
  .closing-inner p:not(.brand-kicker) {
    max-width: 420px;
    color: var(--brand-night-muted);
    font-size: 14px;
    margin: 22px 0 0;
  }
  .closing-actions {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    flex-shrink: 0;
  }
  .closing-actions .brand-link {
    color: var(--brand-night-text);
  }
  .closing-actions small {
    font-size: 10px;
    color: var(--brand-night-muted);
  }
  .closing-actions small a {
    color: var(--brand-night-text);
  }
  @media (max-width: 1000px) {
    .hero-bottom {
      align-items: flex-start;
      flex-direction: column;
      gap: 20px;
    }
    .ownership-grid,
    .community {
      gap: 45px;
    }
    .feature-grid {
      gap: 0 24px;
    }
    .feature-grid p {
      min-height: 105px;
    }
    .closing-inner {
      padding: 36px;
    }
  }
  @media (max-width: 700px) {
    .site-header {
      min-height: 76px;
    }
    .site-header nav {
      display: none;
    }
    .hero {
      padding-top: 44px;
    }
    h1 {
      font-size: clamp(43px, 8.8vw, 60px);
    }
    .hero-heading .brand-kicker {
      font-size: 8px;
    }
    .hero-bottom > p {
      font-size: 14px;
    }
    .desktop-break {
      display: none;
    }
    .hero-actions {
      flex-wrap: wrap;
      gap: 18px;
    }
    .hero-proof {
      gap: 8px 18px;
      font-size: 8px;
      margin-bottom: 28px;
    }
    .preview-caption {
      font-size: 9px;
      flex-direction: column;
      gap: 9px;
    }
    .tools,
    .community {
      padding-block: 56px;
    }
    .section-heading {
      display: block;
      margin-bottom: 30px;
    }
    .section-heading > p {
      margin-top: 22px;
    }
    .feature-grid {
      grid-template-columns: 1fr 1fr;
      gap: 0 22px;
    }
    .feature-grid h3 {
      font-size: 19px;
    }
    .feature-grid p {
      min-height: 0;
      font-size: 12px;
    }
    .tool-list {
      font-size: 8px;
    }
    .ownership-grid,
    .community {
      grid-template-columns: 1fr;
      gap: 38px;
    }
    .ownership-grid {
      padding-block: 52px;
    }
    .source-card {
      padding: 28px;
    }
    .closing {
      padding-bottom: 32px;
    }
    .closing-inner {
      flex-direction: column;
      align-items: flex-start;
      padding: 30px 25px;
      gap: 28px;
    }
    .closing-actions {
      align-items: flex-start;
    }
  }
  @media (max-width: 420px) {
    .feature-grid {
      grid-template-columns: 1fr;
    }
    .feature-grid article {
      padding-block: 24px;
    }
    .feature-grid h3 {
      margin-top: 14px;
    }
    .tool-list {
      margin-top: 15px;
    }
  }
</style>

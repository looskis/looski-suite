<script lang="ts">
  import BrandLogo from './BrandLogo.svelte';
  import BrandFooter from './BrandFooter.svelte';
  import Icon from './Icon.svelte';
  import { brand } from '$lib/brand';
  let { children } = $props();
</script>

<div class="brand-surface auth-frame">
  <a class="brand-skip" href="#auth-content">Skip to sign in</a>
  <div class="auth-layout">
    <aside class="auth-story">
      <BrandLogo inverse />
      <div class="story-copy">
        <p class="brand-kicker">A little less scattered.</p>
        <h2>Your people.<br />Your possibilities.<br /><em>Your place.</em></h2>
        <p>
          All the moving parts of your business.<br />A shared home to move them
          forward.
        </p>
        <div class="brand-art" aria-hidden="true">
          <div class="mark-glow"></div>
          <img
            src="/brand/looski-mark.svg"
            alt=""
            width="180"
            height="180"
          /><span class="art-label people"
            ><Icon name="people" size={16} />Relationships</span
          ><span class="art-label projects"
            ><Icon name="board" size={16} />Projects</span
          ><span class="art-label ideas"
            ><Icon name="book" size={16} />Knowledge</span
          >
        </div>
      </div>
      <a class="story-footer" href={brand.website}
        ><span>Open source. Built for your kind of business.</span><span
          >Discover Looski ↗</span
        ></a
      >
    </aside>
    <div class="auth-entry" id="auth-content">
      <div class="entry-note"><span></span>Your workspace is waiting</div>
      <div class="auth-card">{@render children()}</div>
      <p class="ownership-note">
        <Icon name="access" size={15} />Your infrastructure. Your data. Your
        call.
      </p>
    </div>
  </div>
  <BrandFooter compact />
</div>

<style>
  .auth-layout {
    display: grid;
    grid-template-columns: 1.05fr 1fr;
    min-height: calc(100dvh - 80px);
  }
  .auth-story {
    background: var(--brand-glow), var(--brand-night);
    color: var(--brand-night-text);
    padding: 38px clamp(28px, 5.5vw, 90px) 30px;
    display: flex;
    flex-direction: column;
  }
  .story-copy {
    margin-block: auto;
    padding: 55px 0 18px;
  }
  .brand-kicker {
    color: var(--brand-mint);
    margin: 0 0 20px;
  }
  h2 {
    color: var(--brand-night-text);
    font-size: clamp(43px, 4.5vw, 67px);
    line-height: 1.04;
    margin: 0;
  }
  h2 em {
    color: #c8d5df;
  }
  .story-copy > p:not(.brand-kicker) {
    font-size: 15px;
    color: var(--brand-night-muted);
    margin: 24px 0 0;
  }
  .brand-art {
    height: 245px;
    position: relative;
    max-width: 390px;
    display: grid;
    place-items: center;
    margin-top: 20px;
  }
  .brand-art img {
    transform: rotate(-7deg);
    opacity: 0.85;
  }
  .mark-glow {
    position: absolute;
    inset: 10% 10%;
    background: conic-gradient(
      from 30deg,
      #75b4ff20,
      #9ee5ce20,
      #f8d67b15,
      #f3a7c720,
      #be9de620,
      #75b4ff20
    );
    filter: blur(35px);
    border-radius: 50%;
  }
  .art-label {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 12px 15px;
    border: 1px solid var(--brand-night-line);
    border-radius: 10px;
    font-size: 11px;
    color: var(--brand-night-text);
    background: var(--brand-night-raised);
    box-shadow: 0 8px 24px #0003;
  }
  .people {
    left: 0;
    top: 35px;
  }
  .projects {
    right: 0;
    top: 100px;
  }
  .ideas {
    left: 28px;
    bottom: 22px;
  }
  .story-footer {
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--brand-night-muted);
    text-decoration: none;
    font-size: 10px;
    border-top: 1px solid var(--brand-night-line);
    padding-top: 22px;
  }
  .story-footer span:last-child {
    color: var(--brand-night-text);
  }
  .story-footer:hover span:last-child {
    text-decoration: underline;
  }
  .auth-entry {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    padding: 65px 35px;
  }
  .entry-note {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 30px;
    font: 9px var(--brand-mono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--brand-muted);
  }
  .entry-note > span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: #4e8767;
  }
  .auth-card {
    width: 100%;
    max-width: 390px;
  }
  :global(.auth-card main) {
    margin: 0;
    padding: 0;
    max-width: none;
  }
  :global(.auth-card h1) {
    font-size: 35px;
    line-height: 1.15;
    margin: 0 0 14px;
  }
  :global(.auth-card p) {
    font-size: 13px;
    color: var(--brand-muted);
  }
  :global(.auth-card .eyebrow) {
    font: 500 10px var(--brand-mono);
    color: var(--brand-muted);
    letter-spacing: 0.08em;
    margin: 0 0 15px;
  }
  :global(.auth-card label) {
    margin-top: 22px;
    margin-bottom: 8px;
    font-size: 12px;
    font-weight: 500;
  }
  :global(.auth-card button) {
    width: 100%;
    margin-top: 22px;
    justify-content: center;
  }
  :global(.auth-card .help) {
    border-top: 1px solid var(--brand-line);
    padding-top: 20px;
    margin-top: 25px;
    font-size: 11px;
    line-height: 1.7;
  }
  :global(.auth-card a) {
    font-size: 12px;
  }
  :global(.auth-card .error) {
    color: var(--error-text);
  }
  .ownership-note {
    display: flex;
    gap: 7px;
    align-items: center;
    color: var(--brand-muted);
    font-size: 10px;
    margin: 30px 0 0;
  }
  @media (min-width: 1500px) {
    .auth-story {
      padding-inline: max(70px, calc((100vw - 1280px) / 2));
    }
  }
  @media (max-width: 900px) {
    .auth-story {
      padding-inline: 28px;
    }
    h2 {
      font-size: 45px;
    }
    .auth-entry {
      padding-inline: 28px;
    }
    .brand-art {
      max-width: 300px;
    }
    .art-label {
      padding: 10px;
      font-size: 9px;
    }
  }
  @media (max-width: 650px) {
    .auth-layout {
      display: block;
      min-height: auto;
    }
    .auth-story {
      padding: 24px;
    }
    .story-copy,
    .story-footer {
      display: none;
    }
    .auth-entry {
      padding: 35px 24px 40px;
      min-height: calc(100dvh - 220px);
    }
    .entry-note {
      margin-bottom: 28px;
    }
    :global(.auth-card h1) {
      font-size: 32px;
    }
  }
</style>

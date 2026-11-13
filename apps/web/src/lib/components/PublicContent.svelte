<script lang="ts">
  import type { components } from '$lib/api/schema';
  import RichText from './RichText.svelte';
  let {
    page,
    mediaUrl,
  }: {
    page: components['schemas']['PublicPage'];
    mediaUrl: (id: string) => string;
  } = $props();
</script>

<article class="content-page">
  <h1>{page.title}</h1>
  {#if page.authors?.length}
    <div class="toolbar" aria-label="Authors">
      {#each page.authors as author}
        <span
          >{#if author.image_url || author.media_id}<img
              class="author-image"
              src={author.media_id
                ? mediaUrl(author.media_id)
                : (author.image_url ?? '')}
              alt=""
              width="40"
              height="40"
            />{/if}<strong>{author.name}</strong>{#if author.bio}<span>
              · {author.bio}</span
            >{/if}</span
        >
      {/each}
    </div>
  {/if}
  {#if page.published_at}
    <p class="muted">Published {page.published_at.slice(0, 10)}</p>
  {/if}
  {#each page.fields as field (field.key)}
    {@const value = field.value}
    <section data-field={field.key}>
      {#if value.kind === 'rich_text'}
        <RichText blocks={value.blocks} />
      {:else if value.kind === 'text'}
        <p class="text"><strong>{field.label}:</strong> {value.text}</p>
      {:else if value.kind === 'number'}
        <p><strong>{field.label}:</strong> {value.number}</p>
      {:else if value.kind === 'boolean'}
        <p>
          <strong>{field.label}:</strong>
          {value.checked ? 'Yes' : 'No'}
        </p>
      {:else if value.kind === 'date'}
        <p><strong>{field.label}:</strong> {value.date}</p>
      {:else if value.kind === 'url'}
        <p>
          <a href={value.href} rel="noopener noreferrer nofollow"
            >{field.label}</a
          >
        </p>
      {:else if value.kind === 'media'}
        {#if value.media_type.startsWith('image/')}
          <img src={mediaUrl(value.id)} alt={value.alt_text} />
        {:else}
          <p><a href={mediaUrl(value.id)}>{value.title}</a></p>
        {/if}
      {:else if value.kind === 'form'}
        {#if value.public_key}
          <iframe src={`/f/${value.public_key}/embed`} title={field.label}
          ></iframe>
          <p class="muted">
            <a href={`/f/${value.public_key}`}>Open this form on its own page</a
            >
          </p>
        {:else}
          <p class="muted">This form is not currently available.</p>
        {/if}
      {/if}
    </section>
  {/each}
</article>

<style>
  .author-image {
    width: 40px;
    height: 40px;
    object-fit: contain;
    vertical-align: middle;
    margin-right: 0.5rem;
  }
  .content-page {
    overflow-wrap: anywhere;
  }
  .muted {
    color: var(--text-secondary);
  }
  .text {
    white-space: pre-wrap;
  }
  img {
    max-width: 100%;
    height: auto;
  }
  iframe {
    width: 100%;
    min-height: 34rem;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
  }
</style>

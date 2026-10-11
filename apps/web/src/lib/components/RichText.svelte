<script lang="ts">
  import type { components } from '$lib/api/schema';
  type Block = components['schemas']['RichBlock'];
  type Inline = components['schemas']['RichInline'];
  let { blocks }: { blocks: Block[] } = $props();
</script>

<!-- Typed rendering only: every string below is escaped text, and hrefs were
     allow-listed by the server. Never add raw HTML output here. -->
{#snippet inlines(
  items: Inline[],
)}{#each items as item}{#snippet styled()}{#if item.code}<code>{item.text}</code
        >{:else if item.bold && item.italic}<strong><em>{item.text}</em></strong
        >{:else if item.bold}<strong>{item.text}</strong
        >{:else if item.italic}<em>{item.text}</em
        >{:else}{item.text}{/if}{/snippet}{#if item.href}<a
        href={item.href}
        rel="noopener noreferrer nofollow">{@render styled()}</a
      >{:else}{@render styled()}{/if}{/each}{/snippet}

{#each blocks as block}
  {#if block.kind === 'heading'}
    <svelte:element this={`h${block.level + 1}`}
      >{@render inlines(block.inlines)}</svelte:element
    >
  {:else if block.kind === 'paragraph'}
    <p>{@render inlines(block.inlines)}</p>
  {:else if block.kind === 'quote'}
    <blockquote>{@render inlines(block.inlines)}</blockquote>
  {:else if block.kind === 'bullet_list'}
    <ul>
      {#each block.items as item}<li>{@render inlines(item.inlines)}</li>{/each}
    </ul>
  {:else if block.kind === 'ordered_list'}
    <ol>
      {#each block.items as item}<li>{@render inlines(item.inlines)}</li>{/each}
    </ol>
  {/if}
{/each}

<style>
  blockquote {
    margin-left: 0;
    padding-left: 1em;
    border-left: 3px solid var(--line, #ddd);
  }
</style>

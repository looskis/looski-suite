<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  let {
    children,
    class: className,
    ...attributes
  }: HTMLAttributes<HTMLElement> & { children: Snippet } = $props();
</script>

<section {...attributes} class={['ui-panel', className]}>
  {@render children()}
</section>

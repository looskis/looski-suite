<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  let {
    children,
    class: className,
    ...attributes
  }: HTMLAttributes<HTMLDivElement> & { children: Snippet } = $props();
</script>

<div {...attributes} class={['ui-table-frame', className]}>
  {@render children()}
</div>

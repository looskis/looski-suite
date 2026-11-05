<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  let {
    children,
    variant = 'primary',
    type = 'button',
    class: className,
    ...attributes
  }: HTMLButtonAttributes & {
    children?: Snippet;
    variant?: 'primary' | 'secondary';
  } = $props();
</script>

<button
  {...attributes}
  {type}
  class={['ui-button', { secondary: variant === 'secondary' }, className]}
>
  {@render children?.()}
</button>

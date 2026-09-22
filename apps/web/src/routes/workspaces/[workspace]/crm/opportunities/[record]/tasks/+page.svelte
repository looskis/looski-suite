<script lang="ts">
  import CrmTaskLinks from '$lib/components/CrmTaskLinks.svelte';
  let { data, form } = $props();
</script>

<CrmTaskLinks {data} {form} />

<script lang="ts">
  import BookingPage from '$lib/components/BookingPage.svelte';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<BookingPage {data} {form} />

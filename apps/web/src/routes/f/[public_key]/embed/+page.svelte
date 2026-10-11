<script lang="ts">
  import PublicForm from '$lib/components/PublicForm.svelte';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<PublicForm
  published={data.form}
  submissionKey={data.key}
  result={form}
  embedded
/>

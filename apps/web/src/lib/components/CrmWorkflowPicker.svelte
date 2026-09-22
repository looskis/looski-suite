<script lang="ts">
  let {
    isDeal,
    workflow,
    origin = '',
  }: {
    isDeal: boolean;
    origin?: string;
    workflow: {
      parent: string;
      parents: { id: string; name: string }[];
      moreParents: boolean;
      moreStates: boolean;
      nextState?: string | null;
    };
  } = $props();
</script>

<form method="GET" class="inline">
  {#if origin}<input type="hidden" name="origin" value={origin} />{/if}
  <label
    >{isDeal ? 'Deal category' : 'Opportunity pipeline'}<select
      name="parent"
      value={workflow.parent}
      >{#each workflow.parents as parent}<option value={parent.id}
          >{parent.name}</option
        >{/each}</select
    ></label
  ><button class="secondary">Choose workflow</button>
</form>
<p class="muted">
  Choose a workflow before editing the fields below. Changing this selection
  reloads saved values.
</p>
{#if workflow.moreParents || workflow.moreStates}<p>
    Showing the first 100 choices. A specific pipeline or category can be opened
    from Configuration.
  </p>{/if}

{#if workflow.nextState}<p>
    <a
      href={`?parent=${workflow.parent}&states_after=${workflow.nextState}${origin ? `&origin=${encodeURIComponent(origin)}` : ''}`}
      >More state choices →</a
    > — save changes before switching pages.
  </p>{/if}

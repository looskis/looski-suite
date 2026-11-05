<script lang="ts">
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import ContactFields from '$lib/components/ContactFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Contact intake · Looski</title></svelte:head>

<div class="page-heading">
  <h1>Intake review</h1>
  <div class="page-actions">
    <RecordDrawer title="Submit contact input" error={Boolean(form?.message)}>
      <p>The source input is retained even when identity needs review.</p>
      {#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
      <form method="POST" action="?/submit">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.key ?? data.key}
        /><label
          >Source reference<input
            name="source_ref"
            required
            maxlength="200"
            value={form?.input?.source_ref ?? ''}
            placeholder="Website inquiry or import row reference"
          /></label
        ><ContactFields
          allowSharingReview={false}
          contact={form?.input?.proposed}
        /><button>Resolve input</button>
      </form>
    </RecordDrawer>
  </div>
</div>
<section>
  <h2>Intake queue</h2>
  <form method="GET">
    <label
      >State<select name="state" value={data.state}
        ><option value="pending">Needs review</option><option value="resolved"
          >Resolved</option
        ><option value="dismissed">Dismissed</option></select
      ></label
    ><button>Show intake</button>
  </form>
  <ul>
    {#each data.page.items as item}<li>
        <a href={`/workspaces/${data.workspace.id}/intakes/${item.id}`}
          >{item.input.proposed.display_name || 'Unnamed input'}</a
        >
        · {item.source_ref}
      </li>{:else}<li>No intake in this state.</li>{/each}
  </ul>
  {#if data.page.next_cursor}<a
      href={`?${new URLSearchParams({ state: data.state, after: data.page.next_cursor })}`}
      >Next page →</a
    >{/if}
</section>

<script lang="ts">
  import ContactFields from '$lib/components/ContactFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Contact intake · Looski</title></svelte:head>
<header>
  <a href={`/workspaces/${data.workspace.id}/contacts`}>← Contacts</a><span
    >{data.workspace.name}</span
  >
</header>
<h1>Resolve incoming contacts.</h1>
<p>
  Reuse a clear match without replacing trusted data. Review shared identifiers
  and uncertain matches before choosing a person.
</p>
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
<section>
  <h2>Submit contact input</h2>
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
</section>

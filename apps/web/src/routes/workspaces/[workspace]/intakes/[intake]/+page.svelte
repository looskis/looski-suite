<script lang="ts">
  import ContactFields from '$lib/components/ContactFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Review contact input · Looski</title></svelte:head><a
  href={`/workspaces/${data.workspace.id}/intakes`}>← Intake queue</a
>
<h1>{data.intake.input.proposed.display_name || 'Unnamed input'}</h1>
<p>{data.intake.source_ref} · {data.intake.state}</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
{#if data.intake.contact_id}<p>
    <a
      href={`/workspaces/${data.workspace.id}/contacts/${data.intake.contact_id}`}
      >Open associated contact</a
    >
  </p>{/if}
<section>
  <h2>Original input</h2>
  <pre style="white-space:pre-wrap;overflow-wrap:anywhere">{JSON.stringify(
      data.intake.input,
      null,
      2,
    )}</pre>
  {#if data.intake.resolution_reason}<p>
      Review reason: {data.intake.resolution_reason}
    </p>{/if}
</section>
{#if data.intake.state === 'pending'}
  <section>
    <h2>Review possible matches</h2>
    <p>
      Up to 50 permitted candidates are shown. Review the source evidence; a
      shared identifier alone does not establish identity.
    </p>
    <ul>
      {#each data.candidates as candidate}<li>
          <a href={`/workspaces/${data.workspace.id}/contacts/${candidate.id}`}
            >{candidate.display_name}</a
          >{candidate.archived ? ' · Archived' : ''}<br /><code
            >{candidate.id}</code
          >
        </li>{:else}<li>
          No permitted candidates to show. You can enter a known contact ID or
          create a reviewed contact.
        </li>{/each}
    </ul>
    <form method="POST">
      <input type="hidden" name="decision" value="link" /><input
        type="hidden"
        name="idempotency_key"
        value={data.key + '-link'}
      /><input
        type="hidden"
        name="expected_version"
        value={data.intake.version}
      /><label>Contact ID<input name="contact_id" required /></label><label
        >Review reason<textarea name="reason" required maxlength="1000"
        ></textarea></label
      ><button>Link this contact</button>
    </form>
  </section>
  <section>
    <h2>Create a reviewed contact</h2>
    <p>
      Correct invalid identifiers before saving. Confirm shared identifiers only
      when this is a separate person.
    </p>
    <form method="POST">
      <input type="hidden" name="decision" value="create" /><input
        type="hidden"
        name="idempotency_key"
        value={data.key + '-create'}
      /><input
        type="hidden"
        name="expected_version"
        value={data.intake.version}
      /><ContactFields contact={data.intake.input.proposed} /><label
        >Review reason<textarea name="reason" required maxlength="1000"
        ></textarea></label
      ><button>Create and associate contact</button>
    </form>
  </section>
  <section>
    <h2>Dismiss input</h2>
    <p>Keep the original evidence without associating a contact.</p>
    <form method="POST">
      <input type="hidden" name="decision" value="dismiss" /><input
        type="hidden"
        name="idempotency_key"
        value={data.key + '-dismiss'}
      /><input
        type="hidden"
        name="expected_version"
        value={data.intake.version}
      /><label
        >Reason<textarea name="reason" required maxlength="1000"
        ></textarea></label
      ><button class="secondary">Dismiss intake</button>
    </form>
  </section>
{/if}

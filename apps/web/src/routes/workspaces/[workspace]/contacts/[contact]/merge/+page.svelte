<script lang="ts">
  import ContactFields from '$lib/components/ContactFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Review duplicate · Looski Contacts</title></svelte:head>
<a href={`/workspaces/${data.workspace.id}/contacts/${data.survivor.id}`}
  >← {data.survivor.display_name}</a
>
<h1>Review a duplicate.</h1>
<p>
  This contact will survive: <strong>{data.survivor.display_name}</strong>. A
  shared email or phone alone does not mean two records represent the same
  person.
</p>
<form method="GET">
  <label
    >Duplicate contact ID<input
      name="source"
      required
      value={data.source?.id ?? ''}
    /></label
  ><button>Preview merge</button>
</form>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
{#if data.source}
  <section>
    <h2>Compare records</h2>
    <div class="columns">
      {#each [data.survivor, data.source] as contact, index}
        {@const relationships =
          index === 0 ? data.survivorRelationships : data.sourceRelationships}
        {@const tags = index === 0 ? data.survivorTags : data.sourceTags}
        <div>
          <h3>{contact.display_name}</h3>
          <p>{contact.notes || 'No notes'}</p>
          <ul>
            {#each contact.identifiers as identifier}<li>
                {identifier.kind}: {identifier.value}
              </li>{/each}
          </ul>
          <h4>Tags</h4>
          {#if tags}<ul>
              {#each tags as tag}<li>
                  {tag.name}{tag.archived ? ' · Archived' : ''}
                </li>{:else}<li>No tags</li>{/each}
            </ul>{:else}<p>Your role cannot preview tags.</p>{/if}
          <h4>Organization relationships</h4>
          {#if relationships}
            <ul>
              {#each relationships as relationship}<li>
                  {relationship.organization_name}: {relationship.relationship_label ||
                    'No label'}
                  {relationship.job_title ? ` · ${relationship.job_title}` : ''}
                  {relationship.is_primary ? ' · Primary' : ''}
                </li>{:else}<li>No visible relationships</li>{/each}
            </ul>
          {:else}<p>
              Your role cannot preview organization relationships.
            </p>{/if}
          <a href={`/workspaces/${data.workspace.id}/contacts/${contact.id}`}
            >Review or resolve relationships</a
          >
        </div>
      {/each}
    </div>
    <p>
      Identifiers and history are retained. Old contact links resolve to the
      survivor. Resolve conflicting organization labels, titles, or primary
      choices using the links above, then reload this preview. Login identities
      and access do not change.
    </p>
  </section>
  <section>
    <h2>Resolve surviving fields</h2>
    <form method="POST">
      <input
        type="hidden"
        name="idempotency_key"
        value={data.idempotencyKey}
      /><input type="hidden" name="source_id" value={data.source.id} /><input
        type="hidden"
        name="source_version"
        value={data.source.version}
      /><input
        type="hidden"
        name="survivor_version"
        value={data.survivor.version}
      />
      <ContactFields contact={data.survivor} />
      <label class="check"
        ><input type="checkbox" name="confirmed" required /> I verified that these
        records represent the same person and reviewed the surviving fields.</label
      >
      <button>Merge into {data.survivor.display_name}</button>
    </form>
  </section>
{/if}

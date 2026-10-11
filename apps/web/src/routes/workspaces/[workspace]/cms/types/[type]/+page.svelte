<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const kindLabel = (kind: string) =>
    data.kinds.find((k) => k.kind === kind)?.label ?? kind;
</script>

<svelte:head
  ><title>{data.item.name} · Content types · Looski</title></svelte:head
>
<h1>{data.item.name}</h1>
{#if data.item.archived}<p>
    This type is archived: its public pages are offline and it cannot be edited
    until restored.
  </p>{:else}<p>
    Public pages: <a href={`/c/${data.item.public_key}`}
      >/c/{data.item.public_key}</a
    >
  </p>{/if}
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section>
  <h2>Fields</h2>
  <p>
    A field keeps its kind once saved. Removing one never changes what is
    already published or stored in a revision.
  </p>
  <form method="POST" action="?/save">
    <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
    <input type="hidden" name="expected_version" value={data.item.version} />
    <input type="hidden" name="field_count" value={data.item.fields.length} />
    <label
      >Name<input
        name="name"
        required
        maxlength="120"
        value={data.item.name}
      /></label
    >
    <label
      >Description<textarea
        name="description"
        maxlength="2000"
        rows="2"
        value={data.item.description}></textarea></label
    >
    {#each data.item.fields as field, i (field.key)}
      <fieldset>
        <legend>{kindLabel(field.kind)}: {field.label}</legend>
        <input type="hidden" name={`field.${i}.key`} value={field.key} />
        <input type="hidden" name={`field.${i}.kind`} value={field.kind} />
        <label
          >Label<input
            name={`field.${i}.label`}
            required
            maxlength="120"
            value={field.label}
          /></label
        >
        <label
          >Help for editors<input
            name={`field.${i}.help`}
            maxlength="500"
            value={field.help}
          /></label
        >
        <label
          ><input
            type="checkbox"
            name={`field.${i}.required`}
            checked={field.required}
          /> Required to publish</label
        >
        <div class="toolbar">
          <button class="secondary" name="op" value={`up:${i}`}>Move up</button>
          <button class="secondary" name="op" value={`down:${i}`}
            >Move down</button
          >
          <button class="secondary" name="op" value={`remove:${i}`}
            >Remove</button
          >
        </div>
      </fieldset>
    {:else}
      <p>No fields yet. Add the first below.</p>
    {/each}
    <button name="op" value="save">Save changes</button>
    <h3>Add a field</h3>
    <label>Label<input name="new_label" maxlength="120" /></label>
    <label
      >Kind<select name="new_kind">
        {#each data.kinds as kind}<option value={kind.kind}>{kind.label}</option
          >{/each}
      </select></label
    >
    <label
      ><input type="checkbox" name="new_required" /> Required to publish</label
    >
    <button class="secondary" name="op" value="add">Add field</button>
  </form>
</section>
<section>
  <h2>{data.item.archived ? 'Restore' : 'Archive'}</h2>
  {#if !data.item.archived}<p>
      Archiving takes this type's {data.item.published_entries} published entries
      offline until it is restored.
    </p>{/if}
  <form method="POST" action="?/archive">
    <input type="hidden" name="idempotency_key" value={data.key} />
    <input type="hidden" name="expected_version" value={data.item.version} />
    <input
      type="hidden"
      name="archived"
      value={data.item.archived ? 'false' : 'true'}
    />
    <button class="secondary"
      >{data.item.archived ? 'Restore type' : 'Archive type'}</button
    >
  </form>
</section>

<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/cms/entries/${data.entry.id}`,
  );
  const draft = $derived(data.entry.draft);
  const published = $derived(data.entry.published);
  const input = $derived(
    (form as { input?: Record<string, string> } | null)?.input,
  );
  const text = (key: string) =>
    input?.[`value.${key}`] ?? String(draft?.values[key] ?? '');
  const title = $derived(draft?.title ?? published?.title ?? '');
  const editable = $derived(
    !data.entry.archived &&
      !data.entry.type_archived &&
      data.entry.review_state !== 'submitted' &&
      data.writing?.can_edit,
  );
</script>

<svelte:head><title>{title} · Content · Looski</title></svelte:head>
<header>
  <a href={`/workspaces/${data.workspace.id}/cms`}>← Content</a>
</header>
<h1>{title}</h1>
<p>
  {data.entry.type_name} ·
  {#if data.entry.archived}Archived
  {:else if published && data.entry.unpublished_changes}Published, with
    unpublished changes
  {:else if published}Published
  {:else}Draft{/if}
  {#if published}
    · <a href={`/c/${data.entry.public_key}/${published.slug}`}
      >View public page</a
    >
  {/if}
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if draft}
  <nav class="toolbar" aria-label="Entry sections">
    <a href={`${root}/preview`}>Preview draft</a>
    <a href={`${root}/revisions`}>Revisions ({draft.revision_number})</a>
  </nav>
  <section>
    <h2>Draft</h2>
    <form method="POST" action="?/save">
      <input
        type="hidden"
        name="idempotency_key"
        value={(form as { key?: string } | null)?.key ?? data.key}
      />
      <input type="hidden" name="publish_key" value={data.key + '-publish'} />
      <input type="hidden" name="expected_version" value={data.entry.version} />
      <label
        >Title<input
          name="title"
          required
          maxlength="200"
          value={input?.title ?? draft.title}
        /></label
      >
      <label
        >Address<input
          name="slug"
          required
          maxlength="120"
          value={input?.slug ?? draft.slug}
        /></label
      >
      {#each data.type.fields as field (field.key)}
        {@const name = `value.${field.key}`}
        {@const label =
          field.label + (field.required ? ' (required to publish)' : '')}
        {#if field.kind === 'rich_text'}
          <label
            >{label}<textarea
              {name}
              rows="10"
              maxlength="100000"
              value={text(field.key)}></textarea></label
          >
          <small
            >Plain text with # headings, - lists, 1. lists, &gt; quotes,
            **bold**, *italic*, `code` and [links](https://example.com). HTML
            shows as typed.</small
          >
        {:else if field.kind === 'text'}
          <label>{label}<input {name} value={text(field.key)} /></label>
        {:else if field.kind === 'number'}
          <label
            >{label}<input
              {name}
              inputmode="decimal"
              value={text(field.key)}
            /></label
          >
        {:else if field.kind === 'date'}
          <label
            >{label}<input type="date" {name} value={text(field.key)} /></label
          >
        {:else if field.kind === 'url'}
          <label
            >{label}<input type="url" {name} value={text(field.key)} /></label
          >
        {:else if field.kind === 'boolean'}
          <label
            ><input
              type="checkbox"
              {name}
              checked={draft.values[field.key] === true}
            />
            {label}</label
          >
        {:else if field.kind === 'media'}
          <label
            >{label}<select {name}>
              <option value="">None</option>
              {#if text(field.key) && !data.media.some((m) => m.id === text(field.key))}
                <option value={text(field.key)} selected>Current choice</option>
              {/if}
              {#each data.media as item}
                <option value={item.id} selected={item.id === text(field.key)}
                  >{item.title}</option
                >
              {/each}
            </select></label
          >
        {:else if field.kind === 'form'}
          {#if data.forms}
            <label
              >{label}<select {name}>
                <option value="">None</option>
                {#if text(field.key) && !data.forms.some((f) => f.id === text(field.key))}
                  <option value={text(field.key)} selected
                    >Current choice</option
                  >
                {/if}
                {#each data.forms as item}
                  <option value={item.id} selected={item.id === text(field.key)}
                    >{item.name}{item.accepting
                      ? ''
                      : ' (not accepting answers)'}</option
                  >
                {/each}
              </select></label
            >
          {:else}
            <p>
              {label}: Forms is off or not available to you{text(field.key)
                ? ', so the current choice is kept and the public page says the form is unavailable.'
                : '.'}
            </p>
            <input type="hidden" {name} value={text(field.key)} />
          {/if}
        {/if}
        {#if field.help}<small>{field.help}</small>{/if}
      {/each}
      {#if editable}
        <div class="toolbar">
          <button>Save draft</button>
          {#if data.writing?.can_publish && !data.entry.contributor_id}<button
              class="secondary"
              formaction="?/saveAndPublish">Save and publish</button
            >{/if}
        </div>
      {/if}
    </form>
  </section>
{/if}
{#if published}
  <section>
    <h2>Published</h2>
    <p>
      Revision {published.revision_number}, published {published.published_at.slice(
        0,
        19,
      )}, at /{published.slug}.
    </p>
    {#if draft && data.writing?.can_publish}
      <form method="POST" action="?/unpublish">
        <input type="hidden" name="idempotency_key" value={data.key} />
        <input
          type="hidden"
          name="expected_version"
          value={data.entry.version}
        />
        <button class="secondary">Unpublish</button>
      </form>
    {/if}
  </section>
{/if}
{#if draft && data.writing?.can_edit && (!published || data.writing?.can_publish) && data.entry.review_state !== 'submitted'}
  <section>
    <h2>{data.entry.archived ? 'Restore' : 'Archive'}</h2>
    {#if !data.entry.archived && published}<p>
        Archiving takes the public page offline. Restoring leaves the entry
        unpublished until it is published again.
      </p>{/if}
    <form method="POST" action="?/archive">
      <input type="hidden" name="idempotency_key" value={data.key} />
      <input type="hidden" name="expected_version" value={data.entry.version} />
      <input
        type="hidden"
        name="archived"
        value={data.entry.archived ? 'false' : 'true'}
      />
      <button class="secondary"
        >{data.entry.archived ? 'Restore entry' : 'Archive entry'}</button
      >
    </form>
  </section>
{/if}

{#if data.entry.contributor_id && draft}
  <section>
    <h2>Editorial review</h2>
    <p>Status: {data.entry.review_state.replaceAll('_', ' ')}</p>
    {#if data.entry.review_note}<p role="status">
        {data.entry.review_note}
      </p>{/if}
    <p>
      Save and preview your changes before submitting. Submitted revisions stay
      locked until returned or withdrawn.
    </p>
    {#if data.entry.authors?.length}<div class="toolbar" aria-label="Authors">
        {#each data.entry.authors as author}<span
            >{#if author.image_url}<img
                src={author.image_url}
                width="32"
                height="32"
                alt=""
              />{/if}{author.name}</span
          >{/each}
      </div>{/if}
    {#if !data.entry.archived && !data.entry.type_archived}
      <form method="POST" action="?/review">
        <input
          type="hidden"
          name="idempotency_key"
          value={data.key + '-review'}
        />
        <input
          type="hidden"
          name="expected_version"
          value={data.entry.version}
        />
        {#if data.entry.review_state === 'submitted'}
          {#if data.writing?.can_publish}
            <label
              >Feedback<textarea name="note" maxlength="3000"></textarea></label
            >
            <button name="review_action" value="request_changes"
              >Request changes</button
            >
            <button formaction="?/approve"
              >Approve and publish this revision</button
            >
          {:else}<button name="review_action" value="withdraw"
              >Withdraw to edit</button
            >{/if}
        {:else if data.entry.unpublished_changes && data.writing?.can_edit}<button
            name="review_action"
            value="submit">Submit saved revision for review</button
          >{/if}
      </form>
    {/if}
  </section>
{/if}

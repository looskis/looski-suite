<script lang="ts">
  import { onMount } from 'svelte';
  import FormBuilderFields from '$lib/components/FormBuilderFields.svelte';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const key = $derived(form?.key ?? data.key);
  const draft = $derived(
    form?.builder ?? {
      name: data.form.name,
      description: data.form.description,
      confirmation_message: data.form.confirmation_message,
      expected_version: String(data.form.version),
      fields: data.form.draft.fields,
      addition: {
        label: '',
        key: '',
        kind: 'short_text',
        options: '',
        required: false,
      },
    },
  );
  const publicUrl = $derived(`${data.origin}/f/${data.form.public_key}`);
  const escapeAttribute = (text: string) =>
    text
      .replaceAll('&', '&amp;')
      .replaceAll('"', '&quot;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;');
  const snippet = $derived(
    `<iframe src="${publicUrl}/embed" title="${escapeAttribute(data.form.name)}" width="100%" height="640" style="border:0" loading="lazy"></iframe>`,
  );
  const status = $derived(
    data.form.archived
      ? 'Archived'
      : !data.form.published_version
        ? 'Draft: not published yet'
        : data.form.accepting
          ? `Accepting responses on version ${data.form.published_version.number}`
          : `Closed: version ${data.form.published_version.number} is not accepting responses`,
  );
  // Copying needs scripting; the snippet stays selectable without it.
  let canCopy = $state(false);
  let copied = $state(false);
  onMount(() => (canCopy = Boolean(navigator.clipboard)));
  async function copy() {
    await navigator.clipboard.writeText(snippet);
    copied = true;
  }
</script>

<svelte:head><title>{data.form.name} · Forms · Looski</title></svelte:head>
<h1>{data.form.name}</h1>
<p>
  {status}{#if data.form.unpublished_changes && data.form.published_version}
    · This draft has unpublished changes.{/if} · {data.form.submission_count ===
  1
    ? '1 submission'
    : `${data.form.submission_count} submissions`}
</p>
{#if data.notice === 'saved'}<p role="status">Draft saved.</p>{/if}
{#if data.notice === 'published'}<p role="status">
    Published. New responses use this version.
  </p>{/if}
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if form?.stale}<p>
    <a href="?">Discard draft and reload current form</a>
  </p>{/if}
<section>
  <h2>Availability</h2>
  <div class="actions">
    {#if data.form.published_version && !data.form.archived}
      <form method="POST" action="?/accepting">
        <input type="hidden" name="idempotency_key" value={key} />
        <input
          type="hidden"
          name="expected_version"
          value={data.form.version}
        />
        <input
          type="hidden"
          name="accepting"
          value={String(!data.form.accepting)}
        />
        <button class="secondary"
          >{data.form.accepting ? 'Close form' : 'Reopen form'}</button
        >
      </form>
    {/if}
    <form method="POST" action="?/archive">
      <input type="hidden" name="idempotency_key" value={key} />
      <input type="hidden" name="expected_version" value={data.form.version} />
      <input
        type="hidden"
        name="archived"
        value={String(!data.form.archived)}
      />
      <button class="secondary"
        >{data.form.archived ? 'Restore form' : 'Archive form'}</button
      >
    </form>
  </div>
  <p>
    Archiving closes the form and keeps its submissions. Restoring leaves it
    closed until you reopen it.
  </p>
  {#if data.form.published_version}
    <h3>Share</h3>
    <p>
      Public link: <a href={publicUrl}>{publicUrl}</a>. Anyone with this link
      can read the published questions and submit answers; it opens nothing
      else.
    </p>
    <label
      >Embed code<textarea readonly rows="3" value={snippet}></textarea></label
    >
    {#if canCopy}<button class="secondary" type="button" onclick={copy}
        >{copied ? 'Copied' : 'Copy embed code'}</button
      >{/if}
    <p>
      Paste this on any site. If you add a <code>sandbox</code> attribute,
      include
      <code>allow-forms allow-same-origin</code>, or submissions will be
      rejected.
    </p>
  {:else}
    <p>Publish the form to get its public link and embed code.</p>
  {/if}
</section>
<section>
  <h2>Builder</h2>
  {#if data.form.archived}
    <p>Restore this form to edit it.</p>
  {/if}
  <form method="POST" action="?/builder">
    <fieldset class="plain" disabled={data.form.archived}>
      <input type="hidden" name="idempotency_key" value={key} />
      <input
        type="hidden"
        name="expected_version"
        value={draft.expected_version}
      />
      <div class="actions">
        <button name="op" value="save">Save draft</button>
        <button class="secondary" name="op" value="publish"
          >Save and publish</button
        >
      </div>
      <label
        >Name<input
          name="name"
          required
          maxlength="120"
          value={draft.name}
        /></label
      >
      <label
        >Description (shown above the questions)<textarea
          name="description"
          maxlength="2000"
          rows="2"
          value={draft.description}></textarea></label
      >
      <label
        >Confirmation message (shown after submitting)<textarea
          name="confirmation_message"
          maxlength="2000"
          rows="2"
          value={draft.confirmation_message}></textarea></label
      >
      <h3>Questions</h3>
      <FormBuilderFields fields={draft.fields} addition={draft.addition} />
      <div class="actions">
        <button name="op" value="save">Save draft</button>
        <button class="secondary" name="op" value="publish"
          >Save and publish</button
        >
      </div>
    </fieldset>
  </form>
</section>

<style>
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }
  .plain {
    border: 0 !important;
    padding: 0 !important;
    margin: 0 !important;
  }
</style>

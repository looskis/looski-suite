<script lang="ts">
  import { fieldKinds, hasOptions, type Field } from '$lib/form-definition';
  import type { Addition } from '$lib/server/form-builder';
  let { fields, addition }: { fields: Field[]; addition: Addition | null } =
    $props();
  /** Earlier questions whose answers can control this question's visibility. */
  const sources = (index: number) =>
    fields
      .slice(0, index)
      .filter((f) => hasOptions(f.kind) || f.kind === 'checkbox')
      .map((f) => ({
        key: f.key,
        label: f.label,
        values:
          f.kind === 'checkbox'
            ? [
                { key: 'true', label: 'Checked' },
                { key: 'false', label: 'Not checked' },
              ]
            : (f.options ?? []),
      }));
</script>

<input type="hidden" name="field_count" value={fields.length} />
{#each fields as field, i (field.key)}
  {@const choices = sources(i)}
  <fieldset class="question">
    <legend>Question {i + 1} · <code>{field.key}</code></legend>
    <input type="hidden" name={`field.${i}.key`} value={field.key} />
    <label
      >Question<input name={`field.${i}.label`} value={field.label} /></label
    >
    <div class="pair">
      <label
        >Type<select name={`field.${i}.kind`} value={field.kind}
          >{#each fieldKinds as kind}<option value={kind.value}
              >{kind.label}</option
            >{/each}</select
        ></label
      >
      <label class="check"
        ><input
          type="checkbox"
          name={`field.${i}.required`}
          checked={field.required}
        />Required</label
      >
    </div>
    <label
      >Help text<textarea name={`field.${i}.help`} rows="2" value={field.help}
      ></textarea></label
    >
    <div class="options">
      <input
        type="hidden"
        name={`field.${i}.option_count`}
        value={field.options?.length ?? 0}
      />
      {#each field.options ?? [] as option, j (option.key)}
        <div class="option">
          <input
            type="hidden"
            name={`field.${i}.option.${j}.key`}
            value={option.key}
          />
          <label
            >Option {j + 1}<input
              name={`field.${i}.option.${j}.label`}
              value={option.label}
            /></label
          >
          {#if j > 0}<button
              class="secondary"
              name="op"
              value={`option_up:${i}:${j}`}
              aria-label={`Move option ${j + 1} up`}>↑</button
            >{/if}
          <button
            class="secondary"
            name="op"
            value={`remove_option:${i}:${j}`}
            aria-label={`Remove option ${j + 1}`}>Remove</button
          >
        </div>
      {/each}
      <label
        >Add options (one per line; for choice questions)<textarea
          name={`field.${i}.new_options`}
          rows="2"></textarea></label
      >
    </div>
    {#if choices.length}
      <details open={Boolean(field.show_if)}>
        <summary
          >Show only when…{#if field.show_if}
            (conditional){/if}</summary
        >
        <p class="muted">
          Leave empty to always show this question. Choose values from one
          earlier question.
        </p>
        {#each choices as source}
          <p class="source">{source.label}</p>
          {#each source.values as value}
            <label class="check"
              ><input
                type="checkbox"
                name={`field.${i}.show_if`}
                value={`${source.key}=${value.key}`}
                checked={field.show_if?.field === source.key &&
                  field.show_if.equals.includes(value.key)}
              />{value.label}</label
            >
          {/each}
        {/each}
      </details>
    {/if}
    <div class="actions">
      {#if i > 0}<button
          class="secondary"
          name="op"
          value={`up:${i}`}
          aria-label={`Move question ${i + 1} up`}>↑ Move up</button
        >{/if}
      {#if i < fields.length - 1}<button
          class="secondary"
          name="op"
          value={`down:${i}`}
          aria-label={`Move question ${i + 1} down`}>↓ Move down</button
        >{/if}
      <button
        class="secondary"
        name="op"
        value={`remove:${i}`}
        aria-label={`Remove question ${i + 1}`}>Remove</button
      >
    </div>
  </fieldset>
{:else}
  <p class="muted">No questions yet. Add the first one below.</p>
{/each}
{#if addition}
  <fieldset class="question">
    <legend>Add a question</legend>
    <label>Question<input name="add.label" value={addition.label} /></label>
    <div class="pair">
      <label
        >Type<select name="add.kind" value={addition.kind}
          >{#each fieldKinds as kind}<option value={kind.value}
              >{kind.label}</option
            >{/each}</select
        ></label
      >
      <label class="check"
        ><input
          type="checkbox"
          name="add.required"
          checked={addition.required}
        />Required</label
      >
    </div>
    <div class="options">
      <label
        >Options (one per line; for choice questions)<textarea
          name="add.options"
          rows="3"
          value={addition.options}></textarea></label
      >
    </div>
    <label
      >Answer key (optional; lowercase letters, digits and underscores)<input
        name="add.key"
        maxlength="40"
        value={addition.key}
      /></label
    >
    <p class="muted">
      A key identifies this question's answers across versions and stays the
      same when you rename the question.
    </p>
    <button class="secondary" name="op" value="add">Add question</button>
  </fieldset>
{/if}

<style>
  .question {
    display: block;
  }
  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 16px;
    align-items: end;
  }
  .option {
    display: flex;
    align-items: end;
    gap: 8px;
  }
  .option label {
    flex: 1;
    margin: 8px 0;
  }
  .option button {
    margin-bottom: 8px;
  }
  /* Option editing only applies to choice questions; visible without CSS :has. */
  .question:not(
      :has(
        select option[value='choice']:checked,
        select option[value='multi_choice']:checked
      )
    )
    .options {
    display: none;
  }
  .source {
    margin: 12px 0 4px;
    font-size: 13px;
    font-weight: 550;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
</style>

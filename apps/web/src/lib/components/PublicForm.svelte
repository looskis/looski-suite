<script lang="ts">
  import { onMount } from 'svelte';
  import {
    answerName,
    readAnswers,
    visibleFields,
    type AnswerInput,
    type Definition,
    type Field,
  } from '$lib/form-definition';
  let {
    published,
    submissionKey,
    result,
    embedded = false,
  }: {
    published: {
      name: string;
      description: string;
      definition: Definition;
    } | null;
    submissionKey: string;
    result:
      | {
          confirmation?: string;
          message?: string;
          input?: AnswerInput;
          errors?: Record<string, string>;
          key?: string;
        }
      | null
      | undefined;
    embedded?: boolean;
  } = $props();
  // Without scripting every question stays visible and the server decides.
  let enhanced = $state(false);
  let answers = $state<Record<string, unknown>>({});
  let formElement = $state<HTMLFormElement>();
  const shown = $derived(
    new Set(published ? visibleFields(published.definition, answers) : []),
  );
  function refresh() {
    if (formElement && published)
      answers = readAnswers(
        published.definition,
        new FormData(formElement),
      ).answers;
  }
  onMount(() => {
    refresh();
    enhanced = true;
  });
  const fields = $derived(published?.definition.fields ?? []);
  const hint = (field: Field) => {
    const source = fields.find((f) => f.key === field.show_if?.field);
    if (!source) return '';
    const values = field.show_if?.equals.map((value) =>
      source.kind === 'checkbox'
        ? value === 'true'
          ? 'checked'
          : 'not checked'
        : (source.options?.find((o) => o.key === value)?.label ?? value),
    );
    return `Answer only if “${source.label}” is ${values?.join(' or ')}.`;
  };
  const value = (key: string) => {
    const current = result?.input?.[key];
    return typeof current === 'string' ? current : '';
  };
  const chosen = (key: string, option: string) =>
    (result?.input?.[key] ?? []).includes(option);
</script>

<svelte:head><title>{published?.name ?? 'Form'} · Looski</title></svelte:head>
<main class:embedded>
  {#if result?.confirmation !== undefined}
    <div role="status" class="confirmation">
      <h1>Thank you</h1>
      <p>{result.confirmation}</p>
    </div>
  {:else}
    {#if result?.message}<p role="alert">{result.message}</p>{/if}
    {#if published}
      <h1>{published.name}</h1>
      {#if published.description}<p class="description">
          {published.description}
        </p>{/if}
      <form method="POST" bind:this={formElement} onchange={refresh}>
        <input
          type="hidden"
          name="submission_key"
          value={result?.key ?? submissionKey}
        />
        {#each fields as field (field.key)}
          {@const hidden = enhanced && !shown.has(field)}
          {@const error = result?.errors?.[field.key]}
          <fieldset {hidden} disabled={hidden} class:invalid={error}>
            {#if field.kind === 'statement'}
              <p class="statement">{field.label}</p>
              {#if field.help}<p class="help">{field.help}</p>{/if}
            {:else if field.kind === 'checkbox'}
              <label class="check"
                ><input
                  type="checkbox"
                  name={answerName(field.key)}
                  checked={value(field.key) === 'on'}
                /><span
                  >{field.label}{#if field.required}<span class="required">
                      (required)</span
                    >{/if}</span
                ></label
              >
              {#if field.help}<p class="help">{field.help}</p>{/if}
            {:else if field.kind === 'multi_choice'}
              <legend
                >{field.label}{#if field.required}<span class="required">
                    (required)</span
                  >{/if}</legend
              >
              {#if field.help}<p class="help">{field.help}</p>{/if}
              {#each field.options ?? [] as option}
                <label class="check"
                  ><input
                    type="checkbox"
                    name={answerName(field.key)}
                    value={option.key}
                    checked={chosen(field.key, option.key)}
                  /><span>{option.label}</span></label
                >
              {/each}
            {:else}
              <label
                >{field.label}{#if field.required}<span class="required">
                    (required)</span
                  >{/if}
                {#if field.help}<span class="help">{field.help}</span>{/if}
                {#if field.kind === 'long_text'}<textarea
                    name={answerName(field.key)}
                    maxlength="10000"
                    required={field.required && !field.show_if}
                    value={value(field.key)}></textarea>
                {:else if field.kind === 'choice'}<select
                    name={answerName(field.key)}
                    required={field.required && !field.show_if}
                    ><option value="">Choose…</option
                    >{#each field.options ?? [] as option}<option
                        value={option.key}
                        selected={value(field.key) === option.key}
                        >{option.label}</option
                      >{/each}</select
                  >
                {:else}<input
                    name={answerName(field.key)}
                    type={{
                      email: 'email',
                      phone: 'tel',
                      number: 'number',
                      date: 'date',
                    }[field.kind as string] ?? 'text'}
                    step={field.kind === 'number' ? 'any' : undefined}
                    maxlength={field.kind === 'short_text' ? 500 : undefined}
                    required={field.required && !field.show_if}
                    value={value(field.key)}
                  />{/if}</label
              >
            {/if}
            {#if field.show_if && !enhanced}<p class="help">
                {hint(field)}
              </p>{/if}
            {#if error}<p class="field-error">{error}</p>{/if}
          </fieldset>
        {/each}
        <button>Submit</button>
      </form>
    {/if}
  {/if}
  {#if !embedded}<p class="footer">Powered by Looski Suite</p>{/if}
</main>

<style>
  main {
    max-width: 640px;
    margin: 0 auto;
    padding: 48px 20px;
    overflow-wrap: anywhere;
  }
  main.embedded {
    max-width: none;
    padding: 16px;
  }
  .description {
    color: var(--muted);
    white-space: pre-wrap;
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    min-width: 0;
  }
  fieldset[hidden] {
    display: none;
  }
  legend {
    padding: 0;
    margin-top: 18px;
    font-size: 13px;
    font-weight: 550;
  }
  .check {
    display: flex;
    gap: 10px;
    font-weight: 400;
  }
  .help {
    display: block;
    color: var(--muted);
    font-size: 12px;
    font-weight: 400;
    margin: 4px 0;
    white-space: pre-wrap;
  }
  .statement {
    white-space: pre-wrap;
  }
  .required {
    color: var(--muted);
    font-weight: 400;
    margin-left: 0.35em;
  }
  .field-error {
    color: #953b31;
    font-size: 13px;
    margin: 4px 0 0;
  }
  .invalid :global(input:not([type='checkbox'])),
  .invalid :global(select),
  .invalid :global(textarea) {
    border-color: #953b31;
  }
  .confirmation p {
    white-space: pre-wrap;
  }
  .footer {
    color: var(--muted);
    font-size: 11px;
    margin-top: 40px;
  }
</style>

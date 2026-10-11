<script lang="ts">
  import {
    answerName,
    readAnswers,
    visibleFields,
    type Definition,
    type AnswerInput,
  } from '$lib/form-definition';
  let {
    definition,
    input = {},
    errors = {},
  }: {
    definition: Definition;
    input?: AnswerInput;
    errors?: Record<string, string>;
  } = $props();
  let answers = $state<Record<string, unknown>>({});
  let enhanced = $state(false);
  const shown = $derived(new Set(visibleFields(definition, answers)));
  function refresh(event: Event) {
    const form = (event.target as HTMLElement).closest('form');
    if (form) {
      answers = readAnswers(definition, new FormData(form)).answers;
      enhanced = true;
    }
  }
  const value = (key: string) =>
    typeof input[key] === 'string' ? (input[key] as string) : '';
</script>

<div oninput={refresh} onchange={refresh}>
  {#each definition.fields as field}<fieldset
      hidden={enhanced && !shown.has(field)}
    >
      {#if field.kind === 'statement'}<p>{field.label}</p>
      {:else if field.kind === 'checkbox'}<label
          ><input
            type="checkbox"
            name={answerName(field.key)}
            checked={value(field.key) === 'on'}
          />{field.label}{field.required ? ' (required)' : ''}</label
        >
      {:else if field.kind === 'multi_choice'}<legend
          >{field.label}{field.required ? ' (required)' : ''}</legend
        >{#each field.options ?? [] as o}<label
            ><input
              type="checkbox"
              name={answerName(field.key)}
              value={o.key}
              checked={(input[field.key] ?? []).includes(o.key)}
            />{o.label}</label
          >{/each}
      {:else}<label
          >{field.label}{field.required ? ' (required)' : ''}
          {#if field.kind === 'long_text'}<textarea
              name={answerName(field.key)}
              maxlength="10000"
              required={field.required && !field.show_if}
              value={value(field.key)}></textarea>
          {:else if field.kind === 'choice'}<select
              name={answerName(field.key)}
              required={field.required && !field.show_if}
              value={value(field.key)}
              ><option value="">Choose…</option
              >{#each field.options ?? [] as o}<option value={o.key}
                  >{o.label}</option
                >{/each}</select
            >
          {:else}<input
              name={answerName(field.key)}
              type={(
                {
                  email: 'email',
                  phone: 'tel',
                  number: 'number',
                  date: 'date',
                } as Record<string, string>
              )[field.kind] ?? 'text'}
              step={field.kind === 'number' ? 'any' : undefined}
              required={field.required && !field.show_if}
              value={value(field.key)}
            />{/if}</label
        >{/if}
      {#if field.help}<p>{field.help}</p>{/if}{#if field.show_if && !enhanced}<p
        >
          Answer if {definition.fields.find(
            (f) => f.key === field.show_if?.field,
          )?.label} is {field.show_if.equals.join(' or ')}.
        </p>{/if}{#if errors[field.key]}<p role="alert">
          {errors[field.key]}
        </p>{/if}
    </fieldset>{/each}
</div>

<style>
  fieldset {
    border: 0;
    padding: 0;
    min-width: 0;
  }
  fieldset[hidden] {
    display: none;
  }
</style>

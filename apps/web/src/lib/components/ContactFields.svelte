<script lang="ts">
  import type { components } from '$lib/api/schema';
  let {
    contact,
    allowSharingReview = true,
  }: {
    contact?: components['schemas']['ContactInput'];
    allowSharingReview?: boolean;
  } = $props();
</script>

<label
  >Name <input
    name="display_name"
    required
    maxlength="200"
    value={contact?.display_name ?? ''}
    autocomplete="name"
  /></label
>
<div class="columns">
  <label
    >Email addresses <textarea
      name="emails"
      rows="3"
      placeholder="One email address per line"
      autocomplete="off"
      value={contact?.identifiers
        ?.filter((i) => i.kind === 'email')
        .map((i) => i.value)
        .join('\n') ?? ''}></textarea></label
  >
  <label
    >Phone numbers <textarea
      name="phones"
      rows="3"
      placeholder="One international number per line, e.g. +14155550123"
      autocomplete="off"
      value={contact?.identifiers
        ?.filter((i) => i.kind === 'phone')
        .map((i) =>
          'normalized_value' in i
            ? `${i.normalized_value}${i.extension ? ` ext. ${i.extension}` : ''}`
            : i.value,
        )
        .join('\n') ?? ''}></textarea></label
  >
</div>
<label
  >Notes <textarea
    name="notes"
    rows="4"
    maxlength="10000"
    value={contact?.notes ?? ''}></textarea></label
>
{#if allowSharingReview}
  <label class="check"
    ><input
      type="checkbox"
      name="allow_shared"
      checked={contact?.allow_shared ?? false}
    /> I reviewed this person and confirm any shared email or phone belongs on this
    separate contact.</label
  >
{/if}

<script lang="ts">
  import type { components } from '$lib/api/schema';
  let {
    organization,
  }: { organization?: components['schemas']['OrganizationInput'] } = $props();
</script>

<label
  >Organization name<input
    name="name"
    required
    maxlength="200"
    value={organization?.name ?? ''}
    autocomplete="organization"
  /></label
>
<label
  >Website<input
    name="website_url"
    type="url"
    maxlength="2048"
    value={organization?.website_url ?? ''}
    placeholder="https://example.com"
  /></label
>
<label
  >Notes<textarea
    name="notes"
    rows="4"
    maxlength="10000"
    value={organization?.notes ?? ''}></textarea></label
>

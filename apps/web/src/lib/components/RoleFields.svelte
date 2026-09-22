<script lang="ts">
  import type { components } from '$lib/api/schema';
  let {
    role,
    permissions,
  }: {
    role?: components['schemas']['Role'];
    permissions: components['schemas']['Permission'][];
  } = $props();
</script>

<label
  >Role name <input
    name="name"
    value={role?.name ?? ''}
    required
    maxlength="120"
  /></label
>
{#if role}<label class="check"
    ><input type="checkbox" name="archived" checked={role.archived} /> Archived —
    existing assignments remain, but grant no access.</label
  >{/if}
<p class="muted">
  Grants combine for each action. Workspace permits all eligible records;
  assigned permits records owned by the member. Administrative access can change
  all supported grants.
</p>
{#each permissions as permission}
  <fieldset>
    <legend>{permission.action}</legend>
    {#each permission.scopes as scope}
      <label class="check"
        ><input
          type="checkbox"
          name="grants"
          value={`${permission.action}|${scope}`}
          checked={role?.grants.some(
            (g) => g.action === permission.action && g.scope === scope,
          ) ?? false}
        />{scope}</label
      >
    {/each}
  </fieldset>
{/each}

<style>
  fieldset {
    border: 1px solid #cdd8cb;
    border-radius: 7px;
    display: inline-block;
    vertical-align: top;
    margin: 8px 8px 0 0;
    min-width: 220px;
  }
  legend {
    font-size: 13px;
  }
</style>

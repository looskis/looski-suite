<script lang="ts">
  import type { RecipeChoices, RecipeDraft } from '$lib/server/project-recipe';
  let {
    name,
    draft,
    choices,
  }: { name: string; draft?: Partial<RecipeDraft>; choices: RecipeChoices } =
    $props();
</script>

<label>Name<input name="name" required maxlength="120" value={name} /></label>
<label
  >Deal status that marks the milestone<select name="status_id" required>
    {#each choices.statuses as status}<option
        value={status.id}
        selected={draft?.status_id === status.id}>{status.label}</option
      >{/each}
  </select></label
>
<label
  >Project name prefix (optional)<input
    name="name_prefix"
    maxlength="80"
    value={draft?.name_prefix ?? ''}
  /></label
>
<label class="check"
  ><input
    type="checkbox"
    name="include_deal_owner"
    checked={draft?.include_deal_owner ?? true}
  /> Add the deal's owner to the project</label
>
<fieldset>
  <legend>Project members</legend>
  {#each choices.members as member}
    <label class="check"
      ><input
        type="checkbox"
        name="member_ids"
        value={member.id}
        checked={draft?.member_ids?.includes(member.id) ?? false}
      />
      {member.display_name}</label
    >
  {:else}
    <p>Listing members needs permission to manage project members.</p>
  {/each}
</fieldset>

<style>
  .check {
    display: block;
    font-weight: 400;
  }
  .check input {
    width: auto;
  }
</style>

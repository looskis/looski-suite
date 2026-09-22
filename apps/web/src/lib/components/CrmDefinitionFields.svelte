<script lang="ts">
  import type { components } from '$lib/api/schema';
  let {
    kind,
    definition,
    draft,
    parents,
  }: {
    kind: string;
    definition?: components['schemas']['Definition'];
    draft?: components['schemas']['SaveDefinition'];
    parents: components['schemas']['Definition'][];
  } = $props();
</script>

<label
  >Name<input
    name="name"
    required
    maxlength="120"
    value={draft?.name ?? definition?.name ?? ''}
  /></label
>
{#if !definition && (kind === 'stages' || kind === 'statuses')}<label
    >{kind === 'stages' ? 'Pipeline' : 'Category'}<select
      name="parent_id"
      value={draft?.parent_id ?? parents[0]?.id ?? ''}
      required
      >{#if draft?.parent_id && !parents.some((p) => p.id === draft.parent_id)}<option
          value={draft.parent_id}
          >Submitted workflow — review before saving</option
        >{/if}{#each parents as parent}<option value={parent.id}
          >{parent.name}</option
        >{/each}</select
    ></label
  >{/if}
{#if kind === 'stages'}
  {#if definition}<p>
      Outcome: {definition.outcome}. Create a new stage to change its meaning.
    </p>{:else}<label
      >Outcome<select name="outcome" value={draft?.outcome ?? 'open'}
        ><option value="open">Open</option><option value="won">Won</option
        ><option value="lost">Lost</option></select
      ></label
    >{/if}
{/if}
<label
  >Display order<input
    type="number"
    name="position"
    value={draft?.position ?? definition?.position ?? 0}
    min="-2147483648"
    max="2147483647"
    step="1"
  /></label
>
{#if definition}<label class="check"
    ><input
      type="checkbox"
      name="archived"
      checked={draft?.archived ?? definition.archived}
    />Archived — retain history and prevent new assignments.</label
  >{/if}

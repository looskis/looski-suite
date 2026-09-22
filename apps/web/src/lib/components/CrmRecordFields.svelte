<script lang="ts">
  type Choice = { id: string; name: string };
  let {
    isDeal,
    fields,
    workflow,
    origins = [],
    origin = $bindable(''),
    creating = false,
  }: {
    isDeal: boolean;
    fields: {
      title: string;
      notes: string;
      amount: string;
      currency: string;
      date1: string;
      date2: string;
      owner: string;
    };
    workflow: {
      parent: string;
      state: string | null;
      states: Choice[];
      members: Choice[];
    };
    origins?: Choice[];
    origin?: string;
    creating?: boolean;
  } = $props();
</script>

<input type="hidden" name="parent_id" value={workflow.parent} />
<label
  >Title<input
    name="title"
    value={fields.title}
    required
    maxlength="200"
  /></label
>
<label
  >{isDeal ? 'Status' : 'Stage'}<select
    name="state_id"
    value={workflow.state ?? ''}
    required
    >{#if workflow.state && !workflow.states.some((s) => s.id === workflow.state)}<option
        value={workflow.state}
        >Submitted selection — review before saving</option
      >{/if}
    {#each workflow.states as state}<option value={state.id}
        >{state.name}</option
      >{/each}</select
  ></label
>
<label
  >Assignee<select name="owner" value={fields.owner}
    ><option value=""
      >{creating ? 'Assign to me' : 'Keep current assignee'}</option
    >{#if fields.owner && !workflow.members.some((m) => m.id === fields.owner)}<option
        value={fields.owner}>Current or submitted assignee</option
      >{/if}{#each workflow.members as member}<option value={member.id}
        >{member.name}</option
      >{/each}</select
  ></label
>
<div class="columns">
  <label
    >{isDeal ? 'Value' : 'Expected value'}<input
      name="amount"
      value={fields.amount}
      inputmode="decimal"
      placeholder="0.000000"
    /></label
  ><label
    >Currency code<input
      name="currency"
      value={fields.currency}
      pattern="[A-Z]{3}"
      maxlength="3"
      placeholder="USD"
    /></label
  >
</div>
<p class="muted">
  Optional amount and currency go together. Up to six decimal places; no
  currency conversion.
</p>
<div class="columns">
  <label
    >{isDeal ? 'Start date' : 'Expected close date'}<input
      type="date"
      name="date1"
      value={fields.date1}
    /></label
  >{#if isDeal}<label
      >End date<input type="date" name="date2" value={fields.date2} /></label
    >{/if}
</div>
{#if creating && isDeal}<label
    >Originating opportunity<select name="origin" bind:value={origin}
      ><option value="">Direct deal — no prior opportunity</option
      >{#each origins as item}<option value={item.id}>{item.name}</option
        >{/each}</select
    ></label
  >{/if}
<label
  >Notes<textarea name="notes" rows="5" maxlength="10000"
    >{fields.notes}</textarea
  ></label
>

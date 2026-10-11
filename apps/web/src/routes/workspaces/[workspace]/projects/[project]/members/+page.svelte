<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Members · Looski</title></svelte:head>
<h1>Members</h1>
<p>
  A member with an assigned Projects role can open this project only while
  listed here. Membership does not widen what they can see in Tasks or CRM.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<div class="table-wrap">
  <table>
    <thead><tr><th>Name</th><th>State</th><th></th></tr></thead>
    <tbody>
      {#each data.members as member}
        <tr>
          <td>{member.display_name}</td>
          <td>{member.active ? 'Active' : 'Inactive in this workspace'}</td>
          <td>
            {#if data.canManage}
              <form method="POST" action="?/remove" class="inline">
                <input
                  type="hidden"
                  name="idempotency_key"
                  value={`${data.key}:${member.id}`}
                />
                <input type="hidden" name="membership_id" value={member.id} />
                <button class="secondary">Remove</button>
              </form>
            {/if}
          </td>
        </tr>
      {:else}<tr><td colspan="3">No members yet.</td></tr>{/each}
    </tbody>
  </table>
</div>
{#if data.canManage && data.candidates.length}
  <section>
    <h2>Add a member</h2>
    <form method="POST" action="?/add" class="inline">
      <input type="hidden" name="idempotency_key" value={`${data.key}:add`} />
      <label
        >Workspace member<select name="membership_id">
          {#each data.candidates as candidate}<option value={candidate.id}
              >{candidate.display_name}</option
            >{/each}
        </select></label
      >
      <button>Add member</button>
    </form>
  </section>
{/if}

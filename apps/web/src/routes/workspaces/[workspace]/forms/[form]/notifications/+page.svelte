<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const selected = $derived(
    form?.membership_ids ?? data.form.notification_membership_ids,
  );
</script>

<svelte:head><title>Notifications · Forms · Looski</title></svelte:head>
<h1>Notifications</h1>
<p>
  Chosen members receive an email for each new response, with a link to read it
  in the suite. The email holds no answers. Recipients must be able to read
  submissions, and each delivery rechecks that.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section>
  <h2>Recipients for {data.form.name}</h2>
  {#if data.membersHidden}<p>
      You cannot list workspace members, so only current recipients appear here.
    </p>{/if}
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
    <input type="hidden" name="expected_version" value={data.form.version} />
    {#each data.members as member}
      <label class="check"
        ><input
          type="checkbox"
          name="membership_ids"
          value={member.id}
          checked={selected.includes(member.id)}
        />{member.name}</label
      >
    {:else}<p>No members to choose from.</p>{/each}
    <button>Save recipients</button>
  </form>
</section>

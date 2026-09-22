<script lang="ts">
  let { data, form } = $props();
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/access`}>← Workspace access</a
  ><span>Version {data.member.version}</span>
</header>
<h1>{data.member.display_name}</h1>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<section>
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={data.key} /><input
      type="hidden"
      name="expected_version"
      value={data.member.version}
    />
    <label
      >Membership state<select name="state" value={data.member.state}
        ><option value="active">Active</option><option value="suspended"
          >Suspended</option
        ><option value="removed">Removed</option></select
      ></label
    >
    <p>
      Suspension blocks access while preserving assignments. Removal clears all
      assignments and preserves historical record ownership. Restoration uses
      only the roles selected here.
    </p>
    <h2>Roles</h2>
    {#each data.roles as role}<label class="check"
        ><input
          type="checkbox"
          name="role_ids"
          value={role.id}
          checked={data.member.roles.some((r) => r.id === role.id)}
        />{role.name}{role.archived
          ? ' (archived; remove before saving)'
          : ''}</label
      >{/each}
    {#if data.next}<p>
        <a href={`?after=${data.next}`}>More roles →</a> — save changes before changing
        pages.
      </p>{/if}
    <p><button>Save membership</button></p>
  </form>
</section>

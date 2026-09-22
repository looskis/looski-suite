<script lang="ts">
  import RoleFields from '$lib/components/RoleFields.svelte';
  let { data, form } = $props();
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/contacts`}>← Contacts</a><span
    >{data.workspace.name}</span
  >
</header>
<h1>Workspace access</h1>
<p>
  Manage staff roles and workspace membership. Changes apply to the next
  authorized request.
</p>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<section>
  <h2>Roles</h2>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Role</th><th>State</th><th>Grants</th></tr></thead><tbody
        >{#each data.roles.items as role}<tr
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/access/roles/${role.id}`}
                >{role.name}</a
              ></td
            ><td>{role.archived ? 'Archived' : 'Active'}</td><td
              >{role.grants.length}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if data.roles.next_cursor}<p>
      <a href={`?roles_after=${data.roles.next_cursor}`}>More roles →</a>
    </p>{/if}
</section>
<section>
  <h2>Staff members</h2>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Member</th><th>State</th><th>Roles</th></tr></thead><tbody
        >{#each data.members.items as member}<tr
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/access/members/${member.id}`}
                >{member.display_name}</a
              ></td
            ><td>{member.state}</td><td
              >{member.roles.map((r) => r.name).join(', ') || 'No roles'}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if data.members.next_cursor}<p>
      <a href={`?members_after=${data.members.next_cursor}`}>More members →</a>
    </p>{/if}
</section>
<section>
  <h2>Create role</h2>
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={data.key} /><RoleFields
      permissions={data.permissions}
    />
    <p><button>Create role</button></p>
  </form>
</section>

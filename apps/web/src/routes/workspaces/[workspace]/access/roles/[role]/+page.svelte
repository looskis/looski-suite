<script lang="ts">
  import RoleFields from '$lib/components/RoleFields.svelte';
  let { data, form } = $props();
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/access`}>← Workspace access</a
  ><span>Version {data.role.version}</span>
</header>
<h1>{data.role.name}</h1>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<section>
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={data.key} /><input
      type="hidden"
      name="expected_version"
      value={data.role.version}
    /><RoleFields role={data.role} permissions={data.permissions} />
    <p><button>Save role</button></p>
  </form>
</section>

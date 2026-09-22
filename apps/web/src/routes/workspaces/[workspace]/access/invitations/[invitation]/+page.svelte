<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<a href={`/workspaces/${data.workspace.id}/access/invitations`}>← Invitations</a
>
<h1>{data.invitation.email}</h1>
<p>{data.invitation.state} · Expires {data.invitation.expires_at}</p>
<p>Invited roles: {data.invitation.roles.map((r) => r.name).join(', ')}</p>
{#if form?.message}<p role="status">{form.message}</p>{/if}
{#if data.link}<label
    >Private invitation link <textarea readonly rows="5" value={data.link}
    ></textarea></label
  >
  <p>
    Share this link with the intended recipient. The link alone does not grant
    workspace access.
  </p>{/if}
{#if ['pending', 'expired'].includes(data.invitation.state)}<form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.key ?? data.key}
    /><input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.invitation.version}
    /><button name="intent" value="revoke">Revoke invitation</button>
    {#if data.invitation.state === 'pending' && data.mailEnabled}<button
        name="intent"
        value="send">Send invitation email</button
      >{/if}
  </form>{/if}

{#if form?.job}<a href={`/workspaces/${data.workspace.id}/jobs/${form.job}`}
    >View email delivery</a
  >{/if}

<script lang="ts">
  let { data, form } = $props();
  const selected = $derived(
    form?.draft?.action ?? (data.job.state === 'queued' ? 'cancel' : 'resume'),
  );
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/jobs`}>← Background work</a>
</header>
<h1>{data.job.kind}</h1>
<p>State: {data.job.state} · Attempts in this run: {data.job.attempts} of 5</p>
{#if data.job.error_code}<p>
    Reason: {data.job.error_code.replaceAll('_', ' ')}
  </p>{/if}
<p>
  Business details remain subject to their own permissions. A resumed job uses
  its original initiating user and current authorization checks. Exhausted work
  starts a new retry run; prior attempts remain in history.
</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>
  <a href={`?reload=${Date.now()}`}>Discard draft and reload saved values</a
  >{/if}
{#if form?.draft || ['queued', 'blocked', 'exhausted'].includes(data.job.state)}<form
    method="POST"
  >
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.job.version}
    /><label
      >Action<select name="action" value={selected}
        >{#if !['resume', 'cancel'].includes(selected)}<option value={selected}
            >{selected} (unsupported)</option
          >{/if}<option value="resume">Resume blocked or exhausted work</option
        ><option value="cancel">Cancel pending work</option></select
      ></label
    ><button>Apply action</button>
  </form>{/if}
<h2>History</h2>
<ol>
  {#each data.history.items as event}<li>
      {event.created_at} · {event.state} · Attempt {event.attempt}{#if event.error_code}
        · {event.error_code.replaceAll('_', ' ')}{/if}
    </li>{/each}
</ol>
{#if data.history.next_cursor}<a href={`?after=${data.history.next_cursor}`}
    >More history →</a
  >{/if}

{#if data.job.context}<p>
    <a
      href={`/workspaces/${data.workspace.id}/crm/deals/${data.job.context.deal_id}`}
      >{data.job.context.deal_title}</a
    >{#if data.job.context.instance_id}
      · <a
        href={`/workspaces/${data.workspace.id}/crm/deals/${data.job.context.deal_id}/coordination/${data.job.context.instance_id}`}
        >View created checklist</a
      >{/if}
  </p>{/if}

{#if data.job.invitation_context}<p>
    <a
      href={`/workspaces/${data.workspace.id}/access/invitations/${data.job.invitation_context.invitation_id}`}
      >Invitation for {data.job.invitation_context.email}</a
    >
  </p>{/if}

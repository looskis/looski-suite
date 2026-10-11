<script lang="ts">
  import { runStatus, type Run } from '$lib/form-definition';
  let { run, workspace }: { run: Run; workspace: string } = $props();
  const status = $derived(runStatus(run));
  const root = $derived(`/workspaces/${workspace}`);
</script>

<strong class:attention={status.attention}>{status.label}</strong>
<small>{status.detail}</small>
{#if run.intake_id}<a href={`${root}/intakes/${run.intake_id}`}>Intake</a>{/if}
{#if run.contact_id}<a href={`${root}/contacts/${run.contact_id}`}>Contact</a
  >{/if}
{#if run.opportunity_id}<a
    href={`${root}/crm/opportunities/${run.opportunity_id}`}>Inquiry</a
  >{/if}
<a href={`${root}/jobs/${run.job_id}`}>Background work</a>

<style>
  strong.attention {
    color: #953b31;
  }
  small {
    display: block;
  }
  a {
    margin-right: 10px;
    font-size: 12px;
  }
</style>

<script lang="ts">
  import RecipeFields from '$lib/components/ProjectRecipeFields.svelte';
  import { runStatus } from '$lib/project-labels';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}`);
  const current = $derived(data.recipe.revision?.configuration);
</script>

<svelte:head><title>{data.recipe.name} · Looski</title></svelte:head>
<header><a href={`${root}/projects/recipes`}>← Deal recipes</a></header>
<h1>{data.recipe.name}</h1>
<p>
  {data.recipe.active ? 'Active' : 'Inactive'}{#if data.recipe.revision}
    · revision {data.recipe.revision.number}{/if}. Runs use the current
  permissions of the member who activated the recipe, so a revoked permission
  shows here as a failed run.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section>
  <h2>{data.recipe.active ? 'Stop new conversions' : 'Start converting'}</h2>
  <form method="POST" action="?/active">
    <input type="hidden" name="idempotency_key" value={`${data.key}:active`} />
    <input type="hidden" name="expected_version" value={data.recipe.version} />
    <input type="hidden" name="active" value={String(!data.recipe.active)} />
    <button class={data.recipe.active ? 'secondary' : ''}
      >{data.recipe.active ? 'Deactivate recipe' : 'Activate recipe'}</button
    >
  </form>
</section>
<section>
  <h2>Configuration</h2>
  <form method="POST" action="?/save">
    <input type="hidden" name="idempotency_key" value={`${data.key}:save`} />
    <input type="hidden" name="expected_version" value={data.recipe.version} />
    <RecipeFields
      name={form?.name ?? data.recipe.name}
      draft={form?.draft ?? current}
      choices={data.choices}
    />
    <button class="secondary">Save new revision</button>
  </form>
</section>
<section>
  <h2>Runs</h2>
  {#if data.runs}
    <div class="table-wrap">
      <table>
        <thead
          ><tr
            ><th>Started</th><th>State</th><th>Deal</th><th>Project</th><th
            ></th></tr
          ></thead
        >
        <tbody>
          {#each data.runs.items as run}
            {@const status = runStatus(run)}
            <tr>
              <td>{run.created_at}</td>
              <td>
                <strong class:attention={status.attention}
                  >{status.label}</strong
                >
                <small>{status.detail}</small>
                <a href={`${root}/jobs/${run.job_id}`}>Background work</a>
              </td>
              <td
                >{#if run.deal_id}<a href={`${root}/crm/deals/${run.deal_id}`}
                    >Open deal</a
                  >{:else}—{/if}</td
              >
              <td
                >{#if run.project_id}<a
                    href={`${root}/projects/${run.project_id}`}>Open project</a
                  >{:else}—{/if}</td
              >
              <td>
                {#if status.retriable}
                  <form method="POST" action="?/retry">
                    <input
                      type="hidden"
                      name="idempotency_key"
                      value={`${data.key}:${run.id}`}
                    />
                    <input type="hidden" name="run_id" value={run.id} />
                    <input
                      type="hidden"
                      name="expected_version"
                      value={run.version}
                    />
                    <button class="secondary">Retry</button>
                  </form>
                {/if}
              </td>
            </tr>
          {:else}<tr
              ><td colspan="5">No deal has reached this status yet.</td></tr
            >{/each}
        </tbody>
      </table>
    </div>
    {#if data.runs.next_cursor}<a href={`?after=${data.runs.next_cursor}`}
        >Next page →</a
      >{/if}
  {:else}<p>Runs are not available right now.</p>{/if}
</section>

<style>
  strong.attention {
    color: #953b31;
  }
  small {
    display: block;
  }
</style>

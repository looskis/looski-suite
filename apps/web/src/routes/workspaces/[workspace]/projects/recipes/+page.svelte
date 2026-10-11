<script lang="ts">
  import RecipeFields from '$lib/components/ProjectRecipeFields.svelte';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/projects`);
</script>

<svelte:head><title>Deal recipes · Looski</title></svelte:head>
<header><a href={root}>← Projects</a></header>
<h1>Deal recipes</h1>
<p>
  Create a project when a deal reaches a status you choose. Each deal converts
  once per recipe, however often it reaches that status. CRM and Projects both
  have to be on for a recipe to run.
</p>
{#if data.page}
  <div class="table-wrap">
    <table>
      <thead><tr><th>Name</th><th>State</th><th>Revision</th></tr></thead>
      <tbody>
        {#each data.page.items as recipe}
          <tr>
            <td><a href={`${root}/recipes/${recipe.id}`}>{recipe.name}</a></td>
            <td>{recipe.active ? 'Active' : 'Inactive'}</td>
            <td>{recipe.revision?.number ?? '—'}</td>
          </tr>
        {:else}<tr><td colspan="3">No recipes yet.</td></tr>{/each}
      </tbody>
    </table>
  </div>
  {#if data.page.next_cursor}<a href={`?after=${data.page.next_cursor}`}
      >Next page →</a
    >{/if}
  <section>
    <h2>Create a recipe</h2>
    {#if form?.message}<p role="alert">{form.message}</p>{/if}
    {#if data.choices.statuses.length}
      <form method="POST" action="?/create">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.key ?? data.key}
        />
        <RecipeFields
          name={form?.name ?? ''}
          draft={form?.draft}
          choices={data.choices}
        />
        <button>Create recipe</button>
      </form>
    {:else}
      <p>
        Choosing a milestone needs CRM to be on and permission to read its deal
        statuses.
      </p>
    {/if}
  </section>
{:else}
  <section>
    <h2>Recipes are not available</h2>
    <p>{data.denied}</p>
  </section>
{/if}

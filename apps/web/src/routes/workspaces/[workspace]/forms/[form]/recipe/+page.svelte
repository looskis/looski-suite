<script lang="ts">
  import { canRetry } from '$lib/form-definition';
  import RecipeRunStatus from '$lib/components/RecipeRunStatus.svelte';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const key = $derived(form?.key ?? data.key);
  const saved = $derived(data.recipe?.revision?.mapping);
  const draft = $derived(
    form?.draft ?? {
      name_fields: saved?.name_fields ?? [],
      email_field: saved?.email_field ?? '',
      phone_field: saved?.phone_field ?? '',
      phone_country: saved?.phone_country ?? '',
      stage: saved ? `${saved.pipeline_id}/${saved.stage_id}` : '',
      owner_membership_id: saved?.owner_membership_id ?? '',
      title_prefix: saved?.title_prefix ?? data.form.name,
    },
  );
  const ofKind = (kind: string) =>
    data.questions.filter((q) => q.kind === kind);
  const pipelines = $derived([...new Set(data.stages.map((s) => s.pipeline))]);
  const stageKnown = $derived(data.stages.some((s) => s.value === draft.stage));
</script>

<svelte:head><title>CRM recipe · Forms · Looski</title></svelte:head>
<h1>CRM inquiry recipe</h1>
<p>
  Turn each new submission into a contact association and one CRM inquiry. The
  original answers are kept unchanged, and public answers never edit an existing
  contact. Respondents are matched on their exact email or phone: one match is
  reused, no match creates a contact, and anything ambiguous waits for review.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if !data.crmAvailable}
  <p role="status">
    CRM choices are unavailable: {data.crmMessage ?? 'CRM may be turned off.'} This
    form keeps collecting answers either way.
  </p>
{/if}
<section>
  <h2>
    Status: {data.recipe?.active
      ? `active, using mapping revision ${data.recipe.revision?.number}`
      : data.recipe
        ? 'not active'
        : 'not configured'}
  </h2>
  {#if data.recipe?.revision}
    <p>
      {data.recipe.active
        ? 'New submissions run this recipe with the current permissions of the member who activated it.'
        : 'Activate it to run it for new submissions. Activation records your authority, so you need every permission a run uses. Earlier submissions are not run.'}
    </p>
    <form method="POST" action="?/activation">
      <input type="hidden" name="idempotency_key" value={key} />
      <input
        type="hidden"
        name="expected_version"
        value={data.recipe.version}
      />
      <input type="hidden" name="active" value={String(!data.recipe.active)} />
      <button class={data.recipe.active ? 'secondary' : ''}
        >{data.recipe.active ? 'Deactivate' : 'Activate'}</button
      >
    </form>
  {/if}
</section>
<section>
  <h2>Mapping</h2>
  {#if !data.form.published_version}
    <p>Publish the form first; mappings use its published questions.</p>
  {:else}
    <form method="POST" action="?/configure">
      <input type="hidden" name="idempotency_key" value={key} />
      <input
        type="hidden"
        name="expected_version"
        value={data.recipe?.version ?? ''}
      />
      <fieldset>
        <legend>Contact name (up to three short-text questions)</legend>
        {#each ofKind('short_text') as question}
          <label class="check"
            ><input
              type="checkbox"
              name="name_fields"
              value={question.key}
              checked={draft.name_fields.includes(question.key)}
            />{question.label}</label
          >
        {:else}<p class="muted">No short-text questions.</p>{/each}
      </fieldset>
      <label
        >Email question<select name="email_field"
          ><option value="">None</option
          >{#each ofKind('email') as question}<option
              value={question.key}
              selected={draft.email_field === question.key}
              >{question.label}</option
            >{/each}{#if draft.email_field && !ofKind('email').some((q) => q.key === draft.email_field)}<option
              value={draft.email_field}
              selected>{draft.email_field} (not in published version)</option
            >{/if}</select
        ></label
      >
      <label
        >Phone question<select name="phone_field"
          ><option value="">None</option
          >{#each ofKind('phone') as question}<option
              value={question.key}
              selected={draft.phone_field === question.key}
              >{question.label}</option
            >{/each}{#if draft.phone_field && !ofKind('phone').some((q) => q.key === draft.phone_field)}<option
              value={draft.phone_field}
              selected>{draft.phone_field} (not in published version)</option
            >{/if}</select
        ></label
      >
      <label
        >Default phone country (two letters, such as US)<input
          name="phone_country"
          maxlength="2"
          value={draft.phone_country}
        /></label
      >
      <label
        >Starting stage<select name="stage" required
          ><option value="">Choose a stage…</option
          >{#if draft.stage && !stageKnown}<option value={draft.stage} selected
              >Current stage (no longer listed)</option
            >{/if}{#each pipelines as pipeline}<optgroup label={pipeline}
              >{#each data.stages.filter((s) => s.pipeline === pipeline) as stage}<option
                  value={stage.value}
                  selected={draft.stage === stage.value}>{stage.name}</option
                >{/each}</optgroup
            >{/each}</select
        ></label
      >
      <label
        >Owner (optional)<select name="owner_membership_id"
          ><option value="">Member who activated the recipe</option
          >{#each data.members as member}<option
              value={member.id}
              selected={draft.owner_membership_id === member.id}
              >{member.name}</option
            >{/each}</select
        ></label
      >
      <label
        >Inquiry title prefix<input
          name="title_prefix"
          required
          maxlength="120"
          value={draft.title_prefix}
        /></label
      >
      <button>Save mapping</button>
    </form>
  {/if}
</section>
{#if data.recipe}
  <section>
    <h2>Runs</h2>
    {#if !data.runs}
      <p>You need permission to read submissions to see runs.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead
            ><tr><th>Started</th><th>Status</th><th>Mapping</th><th></th></tr
            ></thead
          >
          <tbody>
            {#each data.runs.items as run}
              <tr>
                <td
                  ><a
                    href={`/workspaces/${data.workspace.id}/forms/${data.form.id}/submissions/${run.submission_id}`}
                    >{run.created_at}</a
                  ></td
                >
                <td><RecipeRunStatus {run} workspace={data.workspace.id} /></td>
                <td>Revision {run.revision_number}</td>
                <td>
                  {#if canRetry(run)}
                    <form method="POST" action="?/retry">
                      <input type="hidden" name="idempotency_key" value={key} />
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
            {:else}<tr><td colspan="4">No runs yet.</td></tr>{/each}
          </tbody>
        </table>
      </div>
      {#if data.runs.next_cursor}<a href={`?after=${data.runs.next_cursor}`}
          >Next page →</a
        >{/if}
    {/if}
  </section>
{/if}

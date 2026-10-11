<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const hours = (ms: number) => (ms / 3600000).toFixed(2);
</script>

<svelte:head><title>Time tracking · Looski</title></svelte:head>
<header><span>{data.workspace.name}</span></header>
<h1>Time tracking</h1>
<p>Track your work, keep billable time clear, and watch your budgets.</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if data.entries}
  <section>
    <h2>Record time</h2>
    <form method="POST" action="?/save">
      <input type="hidden" name="idempotency_key" value={data.key} />
      <label
        >Description<input
          name="description"
          required
          maxlength="2000"
        /></label
      >
      <div class="inline">
        <label
          >Work date<input
            name="work_on"
            type="date"
            required
            value={new Date().toISOString().slice(0, 10)}
          /></label
        >
        <label
          >Project<select name="project_id"
            ><option value="">Unassigned work</option
            >{#each data.projects as p}<option value={p.id}>{p.name}</option
              >{/each}</select
          ></label
        >
        <label
          >Minutes<input
            name="minutes"
            type="number"
            min="0"
            step="any"
            value="0"
            required
          /></label
        >
      </div>
      <label><input name="billable" type="checkbox" /> Billable</label>
      <div class="inline">
        <button name="mode" value="manual">Add manual time</button><button
          name="mode"
          value="timer"
          class="secondary">Start timer</button
        >
      </div>
      <p>
        Start timers at zero minutes. Work date determines the reporting period,
        including timers that cross midnight.
      </p>
    </form>
  </section>
  <section>
    <h2>Entries and totals</h2>
    <form method="GET" class="inline">
      <label
        >From<input
          name="from_on"
          type="date"
          value={data.query.from_on ?? ''}
        /></label
      ><label
        >Through<input
          name="through_on"
          type="date"
          value={data.query.through_on ?? ''}
        /></label
      ><label
        >Project<select name="project_id" value={data.query.project_id ?? ''}
          ><option value="">All visible time</option
          >{#each data.projects as p}<option value={p.id}>{p.name}</option
            >{/each}</select
        ></label
      ><label
        ><input
          name="archived"
          type="checkbox"
          value="true"
          checked={data.query.archived}
        /> Archived</label
      ><button class="secondary">Filter</button>
    </form>
    {#if data.report}<p>
        <strong>{hours(data.report.duration_ms)} hours</strong> · {hours(
          data.report.billable_ms,
        )} billable · {data.report.count} entries
      </p>
      <p>
        Totals use the same access and filters as entries. Running time is
        measured at page load; refresh for an update.
      </p>{/if}
    {#each data.entries.items as e}
      <article>
        <div class="toolbar">
          <h3>{e.description}</h3>
          <strong>{hours(e.duration_ms)} h</strong>
        </div>
        <p>
          {e.work_on} · {e.state} · {e.billable ? 'Billable' : 'Non-billable'}
        </p>
        {#if e.state !== 'stopped'}<form
            method="POST"
            action="?/transition"
            class="inline"
          >
            <input
              type="hidden"
              name="idempotency_key"
              value={`${data.key}:${e.id}`}
            /><input type="hidden" name="id" value={e.id} /><input
              type="hidden"
              name="expected_version"
              value={e.version}
            />{#if e.state === 'running'}<button
                name="action"
                value="pause"
                class="secondary">Pause</button
              >{:else}<button name="action" value="resume">Resume</button
              >{/if}<button name="action" value="stop">Stop</button>
          </form>
        {:else}<details>
            <summary>Edit entry</summary>
            <form method="POST" action="?/save">
              <input
                type="hidden"
                name="idempotency_key"
                value={`${data.key}:${e.id}`}
              /><input type="hidden" name="id" value={e.id} /><input
                type="hidden"
                name="expected_version"
                value={e.version}
              /><label
                >Description<input
                  name="description"
                  value={e.description}
                  required
                  maxlength="2000"
                /></label
              >
              <div class="inline">
                <label
                  >Work date<input
                    name="work_on"
                    type="date"
                    value={e.work_on}
                    required
                  /></label
                ><label
                  >Minutes<input
                    name="minutes"
                    type="number"
                    min="0"
                    step="any"
                    value={e.duration_ms / 60000}
                    required
                  /></label
                ><label
                  >Project<select name="project_id" value={e.project_id ?? ''}
                    ><option value="">Unassigned work</option
                    >{#each data.projects as p}<option value={p.id}
                        >{p.name}</option
                      >{/each}</select
                  ></label
                >
              </div>
              <label
                ><input name="billable" type="checkbox" checked={e.billable} /> Billable</label
              ><label
                ><input name="archived" type="checkbox" checked={e.archived} /> Archived</label
              ><button>Save entry</button>
            </form>
          </details>{/if}
      </article>
    {:else}<p>No time in this selection.</p>{/each}
    {#if data.entries.next_cursor}<a
        href={`?${new URLSearchParams({
          ...Object.fromEntries(
            Object.entries(data.query)
              .filter(([, v]) => v !== undefined)
              .map(([k, v]) => [k, String(v)]),
          ),
          after: data.entries.next_cursor,
        })}`}>Next page →</a
      >{/if}
  </section>
  <section>
    <h2>Budgets</h2>
    <p>
      Workspace budgets compare all permitted time in their date range. A budget
      without a project includes all work.
    </p>
    {#each data.budgets as b}<article>
        <h3>{b.name}</h3>
        <p>
          {b.from_on} – {b.through_on} · {b.used === null
            ? 'Unavailable'
            : hours(b.used)} of {hours(b.budget_ms)} hours {b.billable_only
            ? '(billable only)'
            : ''}
        </p>
        {#if b.used !== null}<progress
            max={b.budget_ms}
            value={Math.min(b.used, b.budget_ms)}
            aria-label={`${b.name} usage`}
          ></progress>{#if b.used > b.budget_ms}<p role="status">
              Over budget by {hours(b.used - b.budget_ms)} hours
            </p>{/if}{/if}
        <details>
          <summary>Edit budget</summary>{@render budgetFields(b)}
        </details>
      </article>{/each}
    <details><summary>Add budget</summary>{@render budgetFields(null)}</details>
  </section>
{:else}<section>
    <h2>Time tracking is turned off</h2>
    <p>
      Entries and budgets are retained. Running timers continue measuring
      elapsed time.
    </p>
  </section>{/if}
{#snippet budgetFields(b: PageData['budgets'][number] | null)}<form
    method="POST"
    action="?/budget"
  >
    <input
      type="hidden"
      name="idempotency_key"
      value={`${data.key}:budget:${b?.id ?? 'new'}`}
    /><input type="hidden" name="id" value={b?.id ?? ''} /><input
      type="hidden"
      name="expected_version"
      value={b?.version ?? ''}
    /><label
      >Name<input
        name="name"
        value={b?.name ?? ''}
        required
        maxlength="120"
      /></label
    >
    <div class="inline">
      <label
        >From<input
          type="date"
          name="from_on"
          value={b?.from_on ?? ''}
          required
        /></label
      ><label
        >Through<input
          type="date"
          name="through_on"
          value={b?.through_on ?? ''}
          required
        /></label
      ><label
        >Hours<input
          type="number"
          name="hours"
          step="any"
          min="0.001"
          value={(b?.budget_ms ?? 3600000) / 3600000}
          required
        /></label
      ><label
        >Project<select name="project_id" value={b?.project_id ?? ''}
          ><option value="">All work</option>{#each data.projects as p}<option
              value={p.id}>{p.name}</option
            >{/each}</select
        ></label
      >
    </div>
    <label
      ><input
        type="checkbox"
        name="billable_only"
        checked={b?.billable_only ?? false}
      /> Billable time only</label
    ><label
      ><input type="checkbox" name="archived" checked={b?.archived ?? false} /> Archived</label
    ><button>Save budget</button>
  </form>{/snippet}
{#if data.module}<section>
    <h2>Application selection</h2>
    <form method="POST" action="?/module">
      <input type="hidden" name="idempotency_key" value={data.key} /><input
        type="hidden"
        name="expected_version"
        value={data.module.version}
      /><input
        type="hidden"
        name="enabled"
        value={String(!data.module.enabled)}
      /><button class="secondary"
        >{data.module.enabled ? 'Turn off' : 'Turn on'} Time tracking</button
      >
    </form>
  </section>{/if}

<style>
  article {
    padding: 1rem 0;
    border-bottom: 1px solid var(--line, #ddd);
  }
  progress {
    width: 100%;
  }
</style>

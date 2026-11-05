<script lang="ts">
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  let type = $state('text');
  let name = $state('');
</script>

<svelte:head><title>Custom fields · Settings · Looski</title></svelte:head>
<a class="back-link" href={`/workspaces/${data.workspace.id}/settings`}
  >← Settings</a
>
<div class="page-heading">
  <h1>Record fields</h1>
  <RecordDrawer
    title="New field"
    error={Boolean(form?.message && !form.input.expected_version)}
    >{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
    <form method="POST" action="?/save">
      <input type="hidden" name="kind" value={data.kind} /><input
        type="hidden"
        name="idempotency_key"
        value={data.key}
      /><label
        >Name<input
          name="name"
          required
          maxlength="80"
          bind:value={name}
        /></label
      ><label
        >Field key<input
          name="key"
          required
          pattern={'custom_[a-z][a-z0-9_]{0,47}'}
          maxlength="55"
          value={`custom_${name
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, '_')
            .replace(/^_|_$/g, '')
            .slice(0, 48)}`}
        /></label
      ><label
        >Type<select aria-label="Type" name="field_type" bind:value={type}
          >{#each ['text', 'number', 'date', 'boolean', 'select', 'email', 'url'] as option}<option
              value={option}>{option}</option
            >{/each}</select
        ></label
      >{#if type === 'select'}<label
          >Options<textarea
            name="options"
            required
            rows="5"
            placeholder="One option per line"></textarea></label
        >{/if}<button>Create field</button>
    </form></RecordDrawer
  >
</div>
<nav class="view-tabs">
  <a
    href="?kind=contacts"
    aria-current={data.kind === 'contacts' ? 'page' : undefined}>Contacts</a
  ><a
    href="?kind=organizations"
    aria-current={data.kind === 'organizations' ? 'page' : undefined}
    >Organizations</a
  >
</nav>
{#if form?.message && form.input.expected_version}<p class="error" role="alert">
    {form.message}
  </p>{/if}
<section>
  <h2>Custom fields</h2>
  <div class="record-table">
    <table>
      <thead
        ><tr><th>Field</th><th>Type</th><th>Key</th><th>Status</th><th></th></tr
        ></thead
      ><tbody
        >{#each data.fields.filter((f) => f.custom) as field}<tr
            ><td>{field.name}</td><td>{field.field_type}</td><td
              ><code>{field.key}</code></td
            ><td>{field.archived ? 'Archived' : 'Active'}</td><td
              ><RecordDrawer title={`Edit ${field.name}`} label="Edit field"
                ><form method="POST" action="?/save">
                  <input type="hidden" name="kind" value={data.kind} /><input
                    type="hidden"
                    name="key"
                    value={field.key}
                  /><input
                    type="hidden"
                    name="field_type"
                    value={field.field_type}
                  /><input
                    type="hidden"
                    name="version"
                    value={field.version}
                  /><input
                    type="hidden"
                    name="idempotency_key"
                    value={`${data.key}:${field.key}`}
                  /><label
                    >Name<input
                      name="name"
                      value={field.name}
                      required
                      maxlength="80"
                    /></label
                  >{#if field.field_type === 'select'}<label
                      >Options<textarea
                        name="options"
                        rows="6"
                        value={field.options.join('\n')}></textarea></label
                    >
                    <p class="muted">
                      Add options freely. Existing options are retained for
                      records and saved views.
                    </p>{/if}<label class="check"
                    ><input
                      type="checkbox"
                      name="archived"
                      checked={field.archived}
                    />Archive field</label
                  >
                  <p class="muted">
                    Archived fields retain their data and remain available in
                    existing filters and views.
                  </p>
                  <button>Save field</button>
                </form></RecordDrawer
              ></td
            ></tr
          >{:else}<tr
            ><td colspan="5"
              ><div class="empty-state">
                <strong>No custom fields yet</strong>
                <p>Add the information your team needs to track.</p>
              </div></td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
</section>
<section>
  <h2>Standard fields</h2>
  <div class="standard-fields">
    {#each data.fields.filter((f) => !f.custom) as field}<div>
        <strong>{field.name}</strong><span>{field.field_type}</span>
      </div>{/each}
  </div>
</section>

<style>
  .standard-fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0 24px;
  }
  .standard-fields div {
    display: flex;
    justify-content: space-between;
    padding: 12px 0;
    border-bottom: 1px solid var(--border-default);
    font-size: 12px;
  }
  .standard-fields strong {
    font-weight: 500;
  }
  .standard-fields span {
    color: var(--text-secondary);
  }
</style>

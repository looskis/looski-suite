<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}`);
  const site = $derived(data.sites.find((s) => s.id === data.siteId));
</script>

<svelte:head><title>Visitors · Looski</title></svelte:head>
<div class="page-heading"><h1>Visitors</h1></div>
<p>
  Consent-based website visits and optional browser matching. Suggestions are
  evidence for review, never verified identity or permission to access a
  conversation.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if !data.module?.enabled}
  <section>
    <h2>Enable visitor attribution</h2>
    <p>
      Connect a first-party collector after enabling this module. Browser
      fingerprinting stays off until you enable it for a site.
    </p>
    <form method="POST" action="?/module">
      <input type="hidden" name="idempotency_key" value={data.key} /><input
        type="hidden"
        name="expected_version"
        value={data.module?.version ?? 0}
      /><button name="enabled" value="true">Enable visitors</button>
    </form>
  </section>
{:else}
  <details open={Boolean(data.siteId)}>
    <summary>Sites and collection settings</summary>
    <nav aria-label="Visitor sites">
      <a href="?">New site</a>{#each data.sites as s}<a href={`?site=${s.id}`}
          >{s.name}</a
        >{/each}
    </nav>
    <form method="POST" action="?/site" class="settings">
      <input type="hidden" name="idempotency_key" value={data.key} /><input
        type="hidden"
        name="id"
        value={site?.id ?? ''}
      /><input type="hidden" name="version" value={site?.version ?? ''} />
      <label
        >Site name<input
          name="name"
          required
          maxlength="120"
          value={site?.name ?? ''}
        /></label
      >
      <label
        >Allowed origins, one per line<textarea
          name="origins"
          required
          placeholder="https://example.com"
          value={site?.origins.join('\n') ?? ''}></textarea></label
      >
      <label
        ><input
          type="checkbox"
          name="enabled"
          checked={site?.enabled ?? true}
        /> Accept visits with recognition consent</label
      >
      <label
        ><input
          type="checkbox"
          name="fingerprint"
          checked={site?.fingerprint_enabled ?? false}
        /> Collect browser signals and suggest exact matches</label
      >
      <label
        ><input
          type="checkbox"
          name="similarity"
          checked={site?.similarity_enabled ?? false}
        /> Also suggest similar browser signals</label
      >
      <div class="inline">
        <label
          >Minimum similarity<input
            name="threshold"
            type="number"
            min="0.8"
            max="1"
            step="0.01"
            value={site?.threshold ?? 0.95}
          /></label
        ><label
          >Retention, days<input
            name="retention"
            type="number"
            min="7"
            max="180"
            value={site?.retention_days ?? 90}
          /></label
        >
      </div>
      <p>
        Turning off browser signals deletes stored signals and match decisions.
        Old visits and inactive visitors expire within the retention window.
      </p>
      {#if site}<p>Collector site key: <code>{site.public_key}</code></p>{/if}
      <button>Save site</button>
    </form>
  </details>
  {#if data.detail}
    {@const d = data.detail}
    <section>
      <h2>Visitor {d.visitor.id.slice(0, 8)}</h2>
      <p>First seen {d.visitor.first_seen} · Last seen {d.visitor.last_seen}</p>
      {#if d.visitor.contact_id}<p>
          Attributed to <a href={`${root}/contacts/${d.visitor.contact_id}`}
            >contact</a
          >
          · Evidence: {d.visitor.evidence === 'declared_contact'
            ? 'Self-declared details'
            : 'Staff review'}
        </p>{:else}<p>No contact attribution.</p>{/if}
      <details>
        <summary>Review contact attribution</summary>
        <p>
          Choose a contact only with supporting evidence. This does not verify
          the visitor’s identity.
        </p>
        <form method="GET" class="inline">
          <input type="hidden" name="visitor" value={d.visitor.id} /><label
            >Find contact<input name="search" value={data.search} /></label
          ><button>Search</button>
        </form>
        <form method="POST" action="?/link">
          <input type="hidden" name="idempotency_key" value={data.key} /><input
            type="hidden"
            name="visitor"
            value={d.visitor.id}
          /><input
            type="hidden"
            name="version"
            value={d.visitor.version}
          /><label
            >Contact<select name="contact"
              ><option value="">Remove attribution</option
              >{#each data.contacts as c}<option value={c.id}
                  >{c.display_name}</option
                >{/each}</select
            ></label
          ><button>Save reviewed attribution</button>
        </form>
      </details>
      <h3>Possible matches</h3>
      {#each d.candidates as c}<div class="candidate">
          <a href={`?visitor=${c.candidate_id}`}
            >Visitor {c.candidate_id.slice(0, 8)}</a
          >
          <p>
            {c.reason} · Signal similarity {(c.score * 100).toFixed(1)}% · {c.state}
          </p>
          <form method="POST" action="?/review" class="inline">
            <input
              type="hidden"
              name="idempotency_key"
              value={`${data.key}:${c.candidate_id}`}
            /><input type="hidden" name="visitor" value={d.visitor.id} /><input
              type="hidden"
              name="candidate"
              value={c.candidate_id}
            /><input type="hidden" name="version" value={c.version} /><button
              name="state"
              value="accepted">Accept evidence</button
            ><button name="state" value="rejected" class="secondary"
              >Reject</button
            ><button name="state" value="suggested" class="secondary"
              >Reset review</button
            >
          </form>
        </div>{:else}<p>No matching suggestions.</p>{/each}
      <h3>Recent visits</h3>
      <p>
        Showing up to 1,000 recent events. <a
          href={`${root}/visitors/${d.visitor.id}/export`}
          >Export retained visitor data</a
        >
      </p>
      <table>
        <thead><tr><th>When</th><th>Page</th><th>Referrer</th></tr></thead
        ><tbody
          >{#each d.events as e}<tr
              ><td>{e.created_at}</td><td>{e.page}</td><td
                >{e.referrer_host ?? 'Direct / unavailable'}</td
              ></tr
            >{/each}</tbody
        >
      </table>
      <details>
        <summary>Delete visitor data</summary>
        <form method="POST" action="?/delete">
          <input type="hidden" name="idempotency_key" value={data.key} /><input
            type="hidden"
            name="visitor"
            value={d.visitor.id}
          /><label
            ><input type="checkbox" name="confirm" required /> Permanently delete
            visits, signals and match evidence for this visitor</label
          ><button class="secondary">Delete visitor data</button>
        </form>
      </details>
    </section>
  {/if}
  <section>
    <h2>Visits {site ? `· ${site.name}` : ''}</h2>
    <table>
      <thead
        ><tr
          ><th>Visitor</th><th>Last seen</th><th>Views</th><th>Suggestions</th
          ><th>Contact</th></tr
        ></thead
      ><tbody
        >{#each data.page?.items ?? [] as v}<tr
            ><td><a href={`?visitor=${v.id}`}>{v.id.slice(0, 8)}</a></td><td
              >{v.last_seen}</td
            ><td>{v.views}</td><td>{v.suggestions}</td><td
              >{#if v.contact_id}<a href={`${root}/contacts/${v.contact_id}`}
                  >Contact</a
                >{:else}Unattributed{/if}</td
            ></tr
          >{:else}<tr
            ><td colspan="5">No consented visits collected yet.</td></tr
          >{/each}</tbody
      >
    </table>
    {#if data.page?.next_cursor}<a
        href={`?after=${data.page.next_cursor}${data.siteId ? `&site=${data.siteId}` : ''}`}
        >Next page</a
      >{/if}
  </section>
  <details>
    <summary>Pause collection</summary>
    <form method="POST" action="?/module">
      <input type="hidden" name="idempotency_key" value={data.key} /><input
        type="hidden"
        name="expected_version"
        value={data.module.version}
      /><button name="enabled" value="false" class="secondary"
        >Disable visitors</button
      >
    </form>
  </details>
{/if}

<style>
  section,
  details {
    margin-block: 1rem;
  }
  nav,
  .inline {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .settings {
    max-width: 40rem;
  }
  label {
    display: block;
    margin-block: 0.75rem;
  }
  textarea {
    min-height: 5rem;
    width: 100%;
  }
  .candidate {
    padding: 1rem;
    border: 1px solid var(--border-default);
    margin-block: 1rem;
  }
  table {
    width: 100%;
  }
  td,
  th {
    padding: 0.6rem;
    text-align: left;
    overflow-wrap: anywhere;
  }
  code {
    overflow-wrap: anywhere;
  }
</style>

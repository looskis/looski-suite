<script lang="ts">
  import { page as current } from '$app/state';
  import { answerText, runStatus } from '$lib/form-definition';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const root = $derived(current.url.pathname);
  type Submission = PageData['page']['items'][number];
  const preview = (submission: Submission) => {
    const fields =
      data.versions.find((v) => v.id === submission.version_id)?.definition
        .fields ?? [];
    return Object.entries(submission.answers)
      .slice(0, 3)
      .map(([key, value]) => {
        const field = fields.find((f) => f.key === key);
        return `${field?.label ?? key}: ${answerText(field, value)}`;
      })
      .join(' · ');
  };
</script>

<svelte:head><title>Submissions · Forms · Looski</title></svelte:head>
<h1>Submissions</h1>
<p>Original answers, kept with the form version each respondent answered.</p>
<section>
  <div class="toolbar">
    <h2>Newest first</h2>
    <a class="button secondary" href={`${root}/export`}>Download CSV</a>
  </div>
  <div class="table-wrap">
    <table>
      <thead
        ><tr
          ><th>Submitted</th><th>Version</th><th>Answers</th><th>CRM recipe</th
          ></tr
        ></thead
      >
      <tbody>
        {#each data.page.items as submission}
          <tr>
            <td
              ><a href={`${root}/${submission.id}`}>{submission.submitted_at}</a
              ></td
            >
            <td>{submission.version_number}</td>
            <td>{preview(submission)}</td>
            <td
              >{submission.recipe_run
                ? runStatus(submission.recipe_run).label
                : '—'}</td
            >
          </tr>
        {:else}<tr><td colspan="4">No submissions yet.</td></tr>{/each}
      </tbody>
    </table>
  </div>
  {#if data.page.next_cursor}<a href={`?after=${data.page.next_cursor}`}
      >Next page →</a
    >{/if}
</section>

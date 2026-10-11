<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>Summary · Forms · Looski</title></svelte:head>
<h1>Summary</h1>
{#if data.summary}
  <p>
    {data.summary.submissions} submissions across all versions, listed by the questions
    of published version {data.summary.version_number}. Answers from other
    versions count where their question key still exists.
  </p>
  {#each data.summary.fields as field}
    <section>
      <h2>{field.label}</h2>
      <p>{field.answered} answered</p>
      {#if field.options.length}
        <div class="table-wrap">
          <table>
            <thead><tr><th>Answer</th><th>Count</th><th>Share</th></tr></thead>
            <tbody>
              {#each field.options as option}
                <tr>
                  <td>{option.label}</td>
                  <td>{option.count}</td>
                  <td
                    ><progress
                      max={Math.max(field.answered, 1)}
                      value={option.count}
                      aria-label={`${option.label}: ${option.count} of ${field.answered}`}
                    ></progress></td
                  >
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>
  {/each}
{:else}<p>{data.message}</p>{/if}

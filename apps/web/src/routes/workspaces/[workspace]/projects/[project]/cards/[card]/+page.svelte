<script lang="ts">
  import RichText from '$lib/components/RichText.svelte';
  import { taskStatuses } from '$lib/project-labels';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const task = $derived(data.card.task);
  const locked = $derived(data.project?.archived ?? false);
  const people = $derived(
    data.card.assignee_in_project
      ? data.members
      : [
          {
            id: task.assigned_membership_id,
            display_name: 'Current assignee, outside this project',
          },
          ...data.members,
        ],
  );
</script>

<svelte:head><title>{task.title} · Looski</title></svelte:head>
<h1>{task.title}</h1>
<p>
  <a href={`/workspaces/${data.workspace.id}/tasks/${task.id}`}>Open in Tasks</a
  >
  to edit its title, notes or type, or to see its history.
</p>
{#if task.notes}<p class="notes">{task.notes}</p>{/if}
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if !locked && !task.archived}
  <section>
    <h2>Plan</h2>
    <form method="POST" action="?/schedule">
      <input type="hidden" name="idempotency_key" value={`${data.key}:plan`} />
      <input type="hidden" name="expected_version" value={task.version} />
      <label
        >Assignee<select name="assigned_membership_id">
          {#each people as person}<option
              value={person.id}
              selected={person.id === task.assigned_membership_id}
              >{person.display_name}</option
            >{/each}
        </select></label
      >
      <label
        >Deadline<input
          name="due_on"
          type="date"
          value={task.due_on ?? ''}
        /></label
      >
      <label
        >Status<select name="status">
          {#each taskStatuses as status}<option
              value={status.value}
              selected={task.status === status.value}>{status.label}</option
            >{/each}
        </select></label
      >
      <button>Save plan</button>
    </form>
  </section>
{/if}
<section>
  <h2>Comments</h2>
  {#if form?.commentMessage}<p role="alert">{form.commentMessage}</p>{/if}
  {#each data.comments.items as comment}
    <article class="comment">
      <header>
        <strong>{comment.author_name}</strong>
        <small
          >{comment.created_at}{#if comment.edited}
            · edited{/if}</small
        >
      </header>
      {#if comment.removed}
        <p class="removed">This comment was removed.</p>
      {:else}
        <RichText blocks={comment.blocks ?? []} />
        {#if !locked}
          {#if comment.mine}
            <details open={form?.editing === comment.id}>
              <summary>Edit</summary>
              <form method="POST" action="?/edit">
                <input
                  type="hidden"
                  name="idempotency_key"
                  value={`${data.key}:edit:${comment.id}`}
                />
                <input type="hidden" name="comment_id" value={comment.id} />
                <input
                  type="hidden"
                  name="expected_version"
                  value={comment.version}
                />
                <label
                  >Comment<textarea
                    name="body"
                    rows="3"
                    maxlength="10000"
                    required
                    value={form?.editing === comment.id
                      ? form?.editBody
                      : comment.body}></textarea></label
                >
                <button class="secondary">Save comment</button>
              </form>
            </details>
          {/if}
          <form method="POST" action="?/remove" class="inline">
            <input
              type="hidden"
              name="idempotency_key"
              value={`${data.key}:remove:${comment.id}`}
            />
            <input type="hidden" name="comment_id" value={comment.id} />
            <input
              type="hidden"
              name="expected_version"
              value={comment.version}
            />
            <button class="secondary">Remove comment</button>
          </form>
        {/if}
      {/if}
    </article>
  {:else}
    <p>No comments yet.</p>
  {/each}
  {#if data.comments.next_cursor}<a href={`?after=${data.comments.next_cursor}`}
      >Later comments →</a
    >{/if}
  {#if !locked}
    <form method="POST" action="?/comment">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? `${data.key}:comment`}
      />
      <label
        >Add a comment<textarea
          name="body"
          rows="3"
          maxlength="10000"
          required
          value={form?.body ?? ''}></textarea></label
      >
      <small
        >Supports **bold**, *italic*, `code`, lists and [links](https://…). HTML
        shows as plain text.</small
      >
      <button>Post comment</button>
    </form>
  {/if}
</section>
{#if !locked}
  <section>
    <h2>Take off this board</h2>
    <p>The task stays in Tasks and its comments are kept here.</p>
    <form method="POST" action="?/removeCard">
      <input type="hidden" name="idempotency_key" value={`${data.key}:off`} />
      <input type="hidden" name="expected_version" value={data.card.version} />
      <button class="secondary">Take off board</button>
    </form>
  </section>
{/if}

<style>
  .comment {
    border-top: 1px solid var(--line, #ddd);
    padding: 10px 0;
    overflow-wrap: anywhere;
  }
  .comment small {
    color: var(--muted);
    margin-left: 6px;
  }
  .removed {
    color: var(--muted);
    font-style: italic;
  }
  .notes {
    white-space: pre-wrap;
  }
</style>

<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const admin = $derived(data.writing?.can_manage_contributors);
</script>

<svelte:head
  ><title>{admin ? 'Contributors' : 'My profile'} · Looski</title></svelte:head
>
<h1>{admin ? 'Contributors' : 'My profile'}</h1>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if admin}
  <p>
    Invite a person through <a href={`/workspaces/${data.workspace.id}/access`}
      >Workspace access</a
    > with the Contributor role. Once they accept, approve a collection here. Contributors
    can only write their own articles.
  </p>
  <form method="POST" action="?/role">
    <input
      type="hidden"
      name="idempotency_key"
      value={data.key + '-role'}
    /><button>Create a contributor role and invite</button>
  </form>
  <section>
    <h2>Approve a contributor</h2>
    <form method="POST" action="?/save">
      <input
        type="hidden"
        name="idempotency_key"
        value={data.key + '-create'}
      />
      <input type="hidden" name="expected_version" value="0" />
      <input type="hidden" name="active" value="true" />
      <label
        >Member<select name="membership_id" required
          >{#each data.members as member}<option value={member.id}
              >{member.display_name}</option
            >{/each}</select
        ></label
      >
      <label
        >Collection<select name="type_id" required
          >{#each data.types as type}<option value={type.id}>{type.name}</option
            >{/each}</select
        ></label
      >
      <label
        >Public name<input
          name="name"
          required
          maxlength="120"
          value={form?.input?.name ?? ''}
        /></label
      >
      <label
        >Bio<textarea name="bio" maxlength="2000"
          >{form?.input?.bio ?? ''}</textarea
        ></label
      >
      <button>Approve access</button>
    </form>
  </section>
{/if}
{#each data.grants as grant}
  <section>
    <h2>
      {data.types.find((t) => t.id === grant.type_id)?.name ?? 'Collection'} · {grant.active
        ? 'Approved'
        : 'Access revoked'}
    </h2>
    <form method="POST" action="?/save">
      <input
        type="hidden"
        name="idempotency_key"
        value={data.key + grant.membership_id + grant.type_id}
      />
      <input type="hidden" name="expected_version" value={grant.version} />
      <input type="hidden" name="membership_id" value={grant.membership_id} />
      <input type="hidden" name="type_id" value={grant.type_id} />
      <label
        >Public name<input
          name="name"
          required
          maxlength="120"
          value={grant.name}
        /></label
      >
      <label
        >Bio<textarea name="bio" maxlength="2000">{grant.bio}</textarea></label
      >
      <label
        >Profile image<select name="photo_media_id"
          ><option value="">None</option>
          {#if grant.photo_media_id && !data.media.some((m) => m.id === grant.photo_media_id)}<option
              value={grant.photo_media_id}
              selected>Current image</option
            >{/if}{#each data.media as media}<option
              value={media.id}
              selected={grant.photo_media_id === media.id}>{media.title}</option
            >{/each}</select
        ></label
      >
      {#if admin}<label
          >Access<select name="active"
            ><option value="true" selected={grant.active}>Approved</option
            ><option value="false" selected={!grant.active}>Revoked</option
            ></select
          ></label
        >{:else}<input type="hidden" name="active" value="true" />{/if}
      <button>Save profile</button>
    </form>
  </section>
{/each}
<p>
  Profile changes apply to newly saved revisions. Published attribution changes
  only after editorial review.
</p>

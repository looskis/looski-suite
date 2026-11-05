<script lang="ts">
  let {
    name,
    href,
    kind = 'person',
  }: {
    name: string;
    href: string;
    kind?: 'person' | 'organization';
  } = $props();
  const initials = $derived(
    name
      .trim()
      .split(/\s+/)
      .map((part) => part[0])
      .slice(0, 2)
      .join('')
      .toUpperCase(),
  );
  const tone = $derived(
    [...name].reduce((sum, char) => sum + char.charCodeAt(0), 0) % 5,
  );
</script>

<a class="record-name" {href} title={name}
  ><span
    class="record-monogram"
    class:organization={kind === 'organization'}
    data-tone={tone}
    aria-hidden="true">{initials || '·'}</span
  ><span>{name}</span></a
>

<style>
  .record-name {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-height: 30px;
    max-width: 100%;
    color: var(--text-primary);
  }
  .record-monogram {
    width: 30px;
    height: 30px;
    flex: 0 0 30px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: #e9e4f5;
    color: #695487;
    font-size: 0.625rem;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .record-name > span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .organization {
    border-radius: 7px;
  }
  [data-tone='1'] {
    background: #e1edf6;
    color: #476d8d;
  }
  [data-tone='2'] {
    background: #eee9df;
    color: #7f683c;
  }
  [data-tone='3'] {
    background: #e0ede6;
    color: #477861;
  }
  [data-tone='4'] {
    background: #f3e3e5;
    color: #965b6b;
  }
</style>

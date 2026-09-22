import { error } from '@sveltejs/kit';
import type { components } from '$lib/api/schema';
type Spec = components['schemas']['CollectionSpec'];
export function exportSelection(
  url: URL,
  collections: Spec[],
): components['schemas']['ExportRequest'] {
  const q = url.searchParams;
  const spec = collections.find(
    (item) => item.collection === (q.get('collection') ?? 'contacts'),
  );
  if (!spec) error(400, 'Choose a supported export collection');
  function id(field: string) {
    const value = q.get(field);
    if (!value) return null;
    if (
      !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(
        value,
      )
    )
      error(400, 'Record IDs must be UUIDs');
    return value;
  }
  function flag(field: string, supported: boolean) {
    const value = q.get(field);
    if (!value) return supported ? false : null;
    if (!supported || !['true', 'false'].includes(value))
      error(400, 'Invalid lifecycle selection');
    return value === 'true';
  }
  const parent_id = id('parent_id');
  if (spec.parent_required !== Boolean(parent_id))
    error(
      400,
      spec.parent_required
        ? 'This collection needs a parent record ID'
        : 'This collection does not use a parent record ID',
    );
  return {
    collection: spec.collection,
    parent_id,
    after: id('after'),
    limit: 25,
    archived: flag('archived', spec.supports_archive_selection),
    ended: flag('ended', spec.supports_ended_selection),
  };
}

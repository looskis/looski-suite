import type { components } from '$lib/api/schema';
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
export type MappingDraft = {
  name_fields: string[];
  email_field: string;
  phone_field: string;
  phone_country: string;
  stage: string;
  owner_membership_id: string;
  title_prefix: string;
  expected_version: string;
};
export function mappingDraft(form: FormData): MappingDraft {
  const t = (k: string) => String(form.get(k) ?? '');
  return {
    name_fields: form.getAll('name_fields').map(String),
    email_field: t('email_field'),
    phone_field: t('phone_field'),
    phone_country: t('phone_country').trim().toUpperCase(),
    stage: t('stage'),
    owner_membership_id: t('owner_membership_id'),
    title_prefix: t('title_prefix'),
    expected_version: t('expected_version'),
  };
}
/** Stage choices carry their pipeline so the pair cannot be mismatched. */
export function mappingInput(
  draft: MappingDraft,
): components['schemas']['ConfigureRecipe'] | null {
  const [pipeline_id, stage_id] = draft.stage.split('/');
  if (
    !uuid.test(pipeline_id ?? '') ||
    !uuid.test(stage_id ?? '') ||
    (draft.owner_membership_id && !uuid.test(draft.owner_membership_id))
  )
    return null;
  return {
    expected_version: draft.expected_version
      ? Number(draft.expected_version)
      : null,
    mapping: {
      name_fields: draft.name_fields,
      email_field: draft.email_field || null,
      phone_field: draft.phone_field || null,
      phone_country: draft.phone_country || null,
      pipeline_id,
      stage_id,
      owner_membership_id: draft.owner_membership_id || null,
      title_prefix: draft.title_prefix,
    },
  };
}

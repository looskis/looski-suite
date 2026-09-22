import type { components } from '$lib/api/schema';
export function organizationForm(
  form: FormData,
): components['schemas']['OrganizationInput'] {
  const website = String(form.get('website_url') ?? '').trim();
  return {
    name: String(form.get('name') ?? ''),
    website_url: website || null,
    notes: String(form.get('notes') ?? ''),
  };
}

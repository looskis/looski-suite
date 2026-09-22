import { error } from '@sveltejs/kit';
import type { components } from '$lib/api/schema';
export function roleInput(form: FormData): components['schemas']['SaveRole'] {
  return {
    name: String(form.get('name') ?? ''),
    grants: form.getAll('grants').map((value) => {
      const [action, scope] = String(value).split('|');
      if (scope !== 'workspace' && scope !== 'assigned')
        error(400, 'Invalid grant scope');
      return { action, scope };
    }),
    archived: form.has('archived'),
    expected_version: form.has('expected_version')
      ? Number(form.get('expected_version'))
      : null,
  };
}

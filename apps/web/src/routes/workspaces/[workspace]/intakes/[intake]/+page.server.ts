import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { contactForm } from '$lib/server/contact-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, intake_id: event.params.intake };
  const result = await client.GET(
    '/api/v1/workspaces/{workspace_id}/contact-intakes/{intake_id}',
    { params: { path } },
  );
  if (!result.data)
    error(
      result.response.status,
      result.error?.message ?? 'Intake unavailable',
    );
  const candidates = await client.GET(
    '/api/v1/workspaces/{workspace_id}/contact-intakes/{intake_id}/candidates',
    { params: { path } },
  );
  if (!candidates.data)
    error(
      candidates.response.status,
      candidates.error?.message ?? 'Candidates unavailable',
    );
  return {
    workspace,
    intake: result.data,
    candidates: candidates.data,
    key: randomUUID(),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const form = await event.request.formData();
    const decision = String(form.get('decision'));
    if (!['link', 'create', 'dismiss'].includes(decision))
      return fail(400, { message: 'Choose a supported review decision' });
    const body = {
      expected_version: Number(form.get('expected_version')),
      decision: decision as 'link' | 'create' | 'dismiss',
      reason: String(form.get('reason') ?? ''),
      contact_id:
        decision === 'link' ? String(form.get('contact_id') ?? '') : null,
      contact: decision === 'create' ? contactForm(form) : null,
    };
    const result = await client.POST(
      '/api/v1/workspaces/{workspace_id}/contact-intakes/{intake_id}/resolve',
      {
        params: {
          path: { workspace_id: workspace.id, intake_id: event.params.intake },
          header: {
            'Idempotency-Key': String(form.get('idempotency_key') ?? ''),
          },
        },
        body,
      },
    );
    if (!result.data)
      return fail(result.response.status, {
        message: result.error?.message ?? 'Could not resolve intake',
      });
    redirect(303, event.url.pathname);
  },
};

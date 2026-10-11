import { error, fail, redirect } from '@sveltejs/kit';
import { randomUUID } from 'node:crypto';
import { workspaceClient, requireSameOrigin } from '$lib/server/session';
import { retryKey } from '$lib/server/project-form';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const { workspace, client } = await workspaceClient(
    event,
    event.params.workspace,
  );
  const path = { workspace_id: workspace.id, id: event.params.appointment };
  const [booking, events] = await Promise.all([
    client.GET(
      '/api/v1/workspaces/{workspace_id}/scheduling/appointments/{id}',
      { params: { path } },
    ),
    client.GET(
      '/api/v1/workspaces/{workspace_id}/scheduling/appointments/{id}/events',
      { params: { path, query: { limit: 100 } } },
    ),
  ]);
  if (!booking.data)
    error(
      booking.response.status,
      booking.error?.message ?? 'Appointment unavailable',
    );
  return {
    workspace,
    booking: booking.data,
    events: events.data?.items ?? [],
    key: randomUUID(),
  };
};
export const actions: Actions = {
  change: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    let starts_at: string | null = String(f.get('starts_at') ?? '') || null;
    if (starts_at) {
      const date = new Date(String(starts_at));
      if (Number.isNaN(date.getTime()))
        return fail(400, { message: 'Enter a valid UTC date and time' });
      starts_at = date.toISOString();
    }
    const r = await client.POST(
      '/api/v1/workspaces/{workspace_id}/scheduling/appointments/{id}/change',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.appointment },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: {
          expected_version: Number(f.get('expected_version')),
          action: String(f.get('action')),
          starts_at: starts_at ? String(starts_at) : null,
        },
      },
    );
    if (!r.data) return fail(r.response.status, { message: r.error?.message });
    redirect(303, event.url.pathname);
  },
  retry: async (event) => {
    requireSameOrigin(event);
    const { workspace, client } = await workspaceClient(
      event,
      event.params.workspace,
    );
    const f = await event.request.formData();
    const r = await client.POST(
      '/api/v1/workspaces/{workspace_id}/scheduling/appointments/{id}/retry',
      {
        params: {
          path: { workspace_id: workspace.id, id: event.params.appointment },
          header: { 'Idempotency-Key': retryKey(f) },
        },
        body: { expected_version: Number(f.get('expected_version')) },
      },
    );
    if (!r.data) return fail(r.response.status, { message: r.error?.message });
    redirect(303, event.url.pathname);
  },
};

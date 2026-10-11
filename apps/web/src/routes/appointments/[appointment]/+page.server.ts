import { randomUUID } from 'node:crypto';
import { error, fail, redirect } from '@sveltejs/kit';
import { publicClient } from '$lib/server/public-form';
import { requireSameOrigin } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const code = event.cookies.get('looski_booking');
  const base = {
    key: randomUUID(),
    date:
      event.url.searchParams.get('date') ||
      new Date().toISOString().slice(0, 10),
  };
  if (!code) return { ...base, booking: null, slots: [] };
  const client = publicClient(event);
  const r = await client.GET('/api/v1/public/scheduling/appointments/{id}', {
    params: {
      path: { id: event.params.appointment },
      header: { 'X-Booking-Key': code },
    },
  });
  if (!r.data) {
    if (r.response.status === 404) return { ...base, booking: null, slots: [] };
    error(r.response.status, 'Appointment temporarily unavailable');
  }
  const slots =
    r.data.state === 'confirmed'
      ? await client.GET('/api/v1/public/scheduling/types/{id}/slots', {
          params: {
            path: { id: r.data.public_key },
            query: { date: base.date },
          },
        })
      : null;
  return { ...base, booking: r.data, slots: slots?.data ?? [] };
};
export const actions: Actions = {
  unlock: async (event) => {
    requireSameOrigin(event);
    const f = await event.request.formData(),
      code = String(f.get('code') ?? '').trim();
    const r = await publicClient(event).GET(
      '/api/v1/public/scheduling/appointments/{id}',
      {
        params: {
          path: { id: event.params.appointment },
          header: { 'X-Booking-Key': code },
        },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: 'Appointment or access code unavailable.',
      });
    event.cookies.set('looski_booking', code, {
      path: event.url.pathname,
      httpOnly: true,
      sameSite: 'strict',
      secure: event.url.protocol === 'https:',
      maxAge: 60 * 60 * 24 * 366,
    });
    redirect(303, event.url.pathname);
  },
  change: async (event) => {
    requireSameOrigin(event);
    const f = await event.request.formData();
    const r = await publicClient(event).POST(
      '/api/v1/public/scheduling/appointments/{id}',
      {
        params: {
          path: { id: event.params.appointment },
          header: {
            'X-Booking-Key': event.cookies.get('looski_booking') ?? '',
            'Idempotency-Key': String(f.get('idempotency_key') ?? ''),
          },
        },
        body: {
          expected_version: Number(f.get('expected_version')),
          action: String(f.get('action')),
          starts_at: String(f.get('starts_at') ?? '') || null,
        },
      },
    );
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not change appointment',
      });
    redirect(303, event.url.pathname);
  },
  lock: async (event) => {
    requireSameOrigin(event);
    event.cookies.delete('looski_booking', { path: event.url.pathname });
    redirect(303, event.url.pathname);
  },
};

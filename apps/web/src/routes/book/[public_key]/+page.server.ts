import { randomBytes } from 'node:crypto';
import { error, fail } from '@sveltejs/kit';
import { publicClient } from '$lib/server/public-form';
import { requireSameOrigin } from '$lib/server/session';
import { readAnswers } from '$lib/form-definition';
import type { Actions, PageServerLoad } from './$types';
export const load: PageServerLoad = async (event) => {
  const client = publicClient(event),
    path = { id: event.params.public_key };
  const type = await client.GET('/api/v1/public/scheduling/types/{id}', {
    params: { path },
  });
  if (!type.data)
    error(type.response.status, 'This booking page is unavailable.');
  const date =
    event.url.searchParams.get('date') ||
    new Intl.DateTimeFormat('en-CA', {
      timeZone: type.data.timezone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    }).format(new Date());
  const slots = await client.GET('/api/v1/public/scheduling/types/{id}/slots', {
    params: { path, query: { date } },
  });
  if (!slots.data)
    error(
      slots.response.status,
      slots.error?.message ?? 'Available times could not be loaded',
    );
  return {
    type: type.data,
    date,
    slots: slots.data,
    code: randomBytes(32).toString('hex'),
  };
};
export const actions: Actions = {
  default: async (event) => {
    requireSameOrigin(event);
    const f = await event.request.formData();
    const client = publicClient(event);
    const code = String(f.get('code') ?? '');
    const type = await client.GET('/api/v1/public/scheduling/types/{id}', {
      params: { path: { id: event.params.public_key } },
    });
    if (!type.data)
      return fail(type.response.status, {
        message: 'This booking page is unavailable.',
        code,
        input: {} as Record<string, string | string[]>,
        errors: {} as Record<string, string>,
        values: { starts_at: '', attendee_name: '', attendee_email: '' },
      });
    const read = type.data.form
      ? readAnswers(type.data.form.definition, f)
      : { answers: {}, input: {}, errors: {} };
    const values = {
      starts_at: String(f.get('starts_at') ?? ''),
      attendee_name: String(f.get('attendee_name') ?? ''),
      attendee_email: String(f.get('attendee_email') ?? ''),
    };
    if (Object.keys(read.errors).length)
      return fail(400, {
        message: 'Check the intake questions.',
        code,
        input: read.input,
        errors: read.errors,
        values,
      });
    const r = await client.POST('/api/v1/public/scheduling/types/{id}/book', {
      params: {
        path: { id: event.params.public_key },
        header: { 'X-Booking-Key': code },
      },
      body: {
        ...values,
        form_version_id: String(f.get('form_version_id') ?? '') || null,
        answers: read.answers,
      },
    });
    if (!r.data)
      return fail(r.response.status, {
        message: r.error?.message ?? 'Could not reserve this time',
        code,
        input: read.input,
        errors: read.errors,
        values,
      });
    const path = '/appointments/' + r.data.id;
    event.cookies.set('looski_booking', code, {
      path,
      httpOnly: true,
      sameSite: 'strict',
      secure: event.url.protocol === 'https:',
      maxAge: 60 * 60 * 24 * 366,
    });
    return { created: true, bookingPath: path, code };
  },
};

// SPDX-License-Identifier: AGPL-3.0-only
import { readFile, writeFile, rename } from 'node:fs/promises';

export class Suite {
  constructor(config) {
    this.config = config;
    this.refreshing = null;
  }
  async token(force = false) {
    if (!this.config.sessionFile)
      return (await readFile(this.config.tokenFile, 'utf8')).trim();
    const saved = JSON.parse(await readFile(this.config.sessionFile, 'utf8'));
    if (!force && saved.expires_at > Date.now() + 60000)
      return saved.access_token;
    if (!this.refreshing)
      this.refreshing = (async () => {
        const response = await fetch(
          `${this.config.suiteUrl}/api/v1/cli/sessions/refresh`,
          {
            method: 'POST',
            redirect: 'error',
            signal: AbortSignal.timeout(12000),
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ refresh_token: saved.refresh_token }),
          },
        );
        if (!response.ok) throw new Error(`suite_${response.status}`);
        const next = await response.json();
        next.expires_at = Date.now() + next.expires_in * 1000;
        const temporary = this.config.sessionFile + '.next';
        await writeFile(temporary, JSON.stringify(next), { mode: 0o600 });
        await rename(temporary, this.config.sessionFile);
        return next.access_token;
      })().finally(() => {
        this.refreshing = null;
      });
    return this.refreshing;
  }
  async request(
    path,
    { method = 'GET', body, key, public: anonymous = false, headers = {} } = {},
  ) {
    const token = anonymous ? null : await this.token();
    const response = await fetch(`${this.config.suiteUrl}/api/v1/${path}`, {
      method,
      redirect: 'error',
      signal: AbortSignal.timeout(12000),
      headers: {
        'Content-Type': 'application/json',
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(key ? { 'Idempotency-Key': key } : {}),
        ...headers,
      },
      ...(body ? { body: JSON.stringify(body) } : {}),
    });
    if (!response.ok) throw new Error(`suite_${response.status}`);
    return response.status === 204 ? null : response.json();
  }
  workspace(path, options) {
    return this.request(`workspaces/${this.config.workspace}/${path}`, options);
  }
  async capture(session, lead, key) {
    const identifiers = [{ kind: 'email', value: lead.email }];
    if (lead.phone)
      identifiers.push({ kind: 'phone', value: lead.phone, country: 'US' });
    // Intake resolves identity without overwriting an existing person's trusted details.
    return this.workspace('contact-intakes', {
      method: 'POST',
      key,
      body: {
        source_ref: `concierge:${session.id}:${key}`,
        proposed: {
          display_name: lead.name,
          identifiers,
          notes: `Looski concierge. ${lead.company ? `Organization: ${lead.company}. ` : ''}Reply requested about: ${session.facts.need || 'AI and open-source tools'}`,
        },
      },
    });
  }
  async slots(date) {
    if (!this.config.bookingType) return { available: false, slots: [] };
    const [info, slots] = await Promise.all([
      this.request(`public/scheduling/types/${this.config.bookingType}`, {
        public: true,
      }),
      this.request(
        `public/scheduling/types/${this.config.bookingType}/slots?date=${encodeURIComponent(date)}`,
        { public: true },
      ),
    ]);
    return { available: true, ...info, slots };
  }
  async cancel(session, key) {
    const path = `public/scheduling/appointments/${session.booking.id}`;
    const headers = { 'X-Booking-Key': session.bookingIntent.accessCode };
    const current = await this.request(path, { public: true, headers });
    if (current.state === 'cancelled') return current;
    return this.request(path, {
      public: true,
      method: 'POST',
      key,
      headers,
      body: {
        expected_version: current.version,
        action: 'cancel',
        starts_at: null,
      },
    });
  }
  async book(session, startsAt, key) {
    if (!this.config.bookingType || !session.lead)
      throw new Error('booking_unavailable');
    const { randomBytes } = await import('node:crypto');
    // Persist the capability before sending. Retries use the exact same code and body.
    session.bookingIntent ||= {
      startsAt,
      key,
      accessCode: randomBytes(32).toString('hex'),
    };
    if (session.bookingIntent.startsAt !== startsAt)
      throw new Error('booking_in_progress');
    return this.request(
      `public/scheduling/types/${this.config.bookingType}/book`,
      {
        public: true,
        method: 'POST',
        key: session.bookingIntent.key,
        headers: { 'X-Booking-Key': session.bookingIntent.accessCode },
        body: {
          starts_at: startsAt,
          attendee_name: session.lead.name,
          attendee_email: session.lead.email,
          answers: {},
        },
      },
    );
  }
}

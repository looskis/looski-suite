// SPDX-License-Identifier: MIT
import { createHmac, randomBytes, timingSafeEqual } from 'node:crypto';
/** Framework-neutral first-party Request -> Response adapter. Mount on one route. */
export function visitorProxy({
  suiteUrl,
  siteKey,
  origins,
  secret,
  fetch: request = globalThis.fetch,
}) {
  const base = new URL(suiteUrl);
  if (
    base.protocol !== 'https:' &&
    !(
      base.protocol === 'http:' &&
      ['localhost', '127.0.0.1'].includes(base.hostname)
    )
  )
    throw new Error('HTTPS Suite URL required');
  if (
    !/^[A-Za-z0-9_-]{22}$/.test(siteKey) ||
    !Array.isArray(origins) ||
    !origins.length ||
    typeof secret !== 'string' ||
    secret.length < 32
  )
    throw new Error('Invalid visitor proxy configuration');
  const sign = (value) =>
    createHmac('sha256', secret)
      .update(`visitor-v1:${siteKey}:${value}`)
      .digest('hex');
  function cookie(req) {
    const secure = new URL(req.url).protocol === 'https:';
    const name = secure ? '__Host-looski_visitor' : 'looski_visitor';
    const value = req.headers
      .get('cookie')
      ?.split(';')
      .map((x) => x.trim())
      .find((x) => x.startsWith(name + '='))
      ?.slice(name.length + 1);
    const parts = value?.match(/^([a-f0-9]{64})\.([a-f0-9]{64})$/);
    const token =
      parts &&
      timingSafeEqual(Buffer.from(sign(parts[1])), Buffer.from(parts[2]))
        ? parts[1]
        : null;
    return { secure, name, token };
  }
  // For an authenticated concierge bridge only; never send this token to JS or an LLM.
  function attribution(req) {
    const { token } = cookie(req);
    return token ? { site_key: siteKey, token } : null;
  }
  async function handle(req) {
    const responseHeaders = {
      'Cache-Control': 'no-store',
      'X-Robots-Tag': 'noindex',
    };
    const reply = (body, status = 200) =>
      Response.json(body, { status, headers: responseHeaders });
    const origin =
      req.headers.get('origin') ||
      (req.method === 'GET' ? new URL(req.url).origin : null);
    if (!origins.includes(origin) || new URL(req.url).origin !== origin)
      return reply({ error: 'Origin not allowed' }, 403);
    const upstream = async (path, body) =>
      request(`${base.origin}/api/v1/public/visitors/${siteKey}${path}`, {
        method: body ? 'POST' : 'GET',
        headers: { Origin: origin, 'Content-Type': 'application/json' },
        body: body ? JSON.stringify(body) : undefined,
        redirect: 'error',
        signal: AbortSignal.timeout(8000),
      });
    try {
      if (req.method === 'GET') {
        const r = await upstream('');
        return r.ok
          ? reply(await r.json())
          : reply({ error: 'Collection unavailable' }, r.status);
      }
      if (req.method !== 'POST')
        return reply({ error: 'Method not allowed' }, 405);
      if (!req.headers.get('content-type')?.startsWith('application/json'))
        return reply({ error: 'JSON required' }, 415);
      const reader = req.body?.getReader();
      if (!reader) return reply({ error: 'Invalid request' }, 400);
      const chunks = [];
      let size = 0;
      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        size += value.length;
        if (size > 12000) {
          await reader.cancel();
          return reply({ error: 'Request too large' }, 413);
        }
        chunks.push(value);
      }
      let input;
      try {
        input = JSON.parse(Buffer.concat(chunks).toString());
      } catch {
        return reply({ error: 'Invalid request' }, 400);
      }
      if (!input || typeof input !== 'object')
        return reply({ error: 'Invalid request' }, 400);
      const c = cookie(req);
      if (input.action === 'withdraw') {
        if (c.token) {
          const r = await upstream('/withdraw', { token: c.token });
          if (!r.ok)
            return reply({ error: 'Withdrawal unavailable; retry' }, 503);
        }
        responseHeaders['Set-Cookie'] =
          `${c.name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0${c.secure ? '; Secure' : ''}`;
        return reply({ accepted: true });
      }
      if (input.action !== 'event' || input.consent !== true)
        return reply({ error: 'Recognition consent required' }, 400);
      const config = await upstream('');
      if (!config.ok)
        return reply({ error: 'Collection unavailable' }, config.status);
      const settings = await config.json();
      const token = c.token || randomBytes(32).toString('hex');
      const r = await upstream('/events', {
        token,
        consent: true,
        event_id: input.event_id,
        page: input.page,
        referrer_host: input.referrer_host ?? null,
        components: settings.fingerprint_enabled
          ? (input.components ?? {})
          : {},
      });
      if (!r.ok) return reply({ error: 'Visit not accepted' }, r.status);
      responseHeaders['Set-Cookie'] =
        `${c.name}=${token}.${sign(token)}; Path=/; HttpOnly; SameSite=Lax; Max-Age=${Math.min(180, settings.retention_days) * 86400}${c.secure ? '; Secure' : ''}`;
      return reply({ accepted: true });
    } catch {
      return reply({ error: 'Visitor service unavailable' }, 503);
    }
  }
  return { handle, attribution };
}

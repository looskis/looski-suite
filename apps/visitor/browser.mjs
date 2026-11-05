// SPDX-License-Identifier: MIT
/** Host-managed consent. No storage, identity lookup, or automatic collection. */
export function createVisitor({
  endpoint = '/api/visitor',
  collectSignals,
  fetch: request = globalThis.fetch,
  location = globalThis.location,
  referrer = () => globalThis.document?.referrer || '',
}) {
  const url = new URL(endpoint, location.href);
  if (url.origin !== location.origin)
    throw new Error('A same-origin visitor proxy is required');
  let consent = false,
    generation = 0,
    queue = Promise.resolve();
  const serialize = (task) => {
    const result = queue.then(task);
    queue = result.catch(() => {});
    return result;
  };
  const post = async (body) => {
    const r = await request(url, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!r.ok) throw new Error('Visitor request unavailable');
    return r.json();
  };
  return {
    setConsent(value) {
      consent = value === true;
      generation++;
    },
    page(path = location.pathname) {
      if (!consent) return Promise.resolve(false);
      const current = generation;
      return serialize(async () => {
        if (!consent || current !== generation) return false;
        const config = await request(url, { credentials: 'same-origin' });
        if (!config.ok) return false;
        const settings = await config.json();
        if (!consent || current !== generation) return false;
        let components = {};
        if (settings.fingerprint_enabled && collectSignals) {
          try {
            components = await collectSignals();
          } catch {
            /* cookie-only attribution remains available */
          }
        }
        if (!consent || current !== generation) return false;
        let referrer_host = null;
        try {
          referrer_host = new URL(referrer()).hostname || null;
        } catch {
          /* no referrer */
        }
        await post({
          action: 'event',
          consent: true,
          event_id: crypto.randomUUID(),
          page: path.split(/[?#]/)[0],
          referrer_host,
          components,
        });
        return true;
      });
    },
    withdraw() {
      consent = false;
      generation++;
      // Wait for earlier requests to finish before deleting their data/cookie.
      return serialize(() => post({ action: 'withdraw' }));
    },
  };
}
/** Pass a lazy import function so Thumbmark never loads before recognition consent.
 * Tested mapping: ThumbmarkJS 1.12.0. Bundle it locally; do not use a remote script.
 */
export function thumbmarkSignals(loadThumbmark) {
  return async () => {
    const { Thumbmark } = await loadThumbmark();
    const client = new Thumbmark({
      logging: false,
      collect_beacon: false,
      simple_request: false,
      api_key: '',
      timeout: 1500,
      include: ['system', 'hardware', 'canvas', 'audio', 'screen', 'locales'],
    });
    const { components: c = {}, error = [] } = await client.get();
    const failed = new Set(error.map((e) => e.component));
    const values = {
      platform: c.system?.platform,
      browser: c.system?.browser?.name,
      major: c.system?.browser?.version?.match(/^\d+/)?.[0],
      gpu:
        typeof c.hardware?.videocard === 'object'
          ? c.hardware.videocard
          : undefined,
      canvas: c.canvas,
      audio: c.audio?.sampleHash,
      screen: c.screen,
      timezone: c.locales?.timezone,
      languages: c.locales?.languages,
      hardware: c.system?.hardwareConcurrency,
    };
    const out = {};
    const source = {
      platform: 'system',
      browser: 'system',
      major: 'system',
      gpu: 'hardware',
      canvas: 'canvas',
      audio: 'audio',
      screen: 'screen',
      timezone: 'locales',
      languages: 'locales',
      hardware: 'system',
    };
    const canonical = (v) =>
      v && typeof v === 'object'
        ? Array.isArray(v)
          ? v.map(canonical)
          : Object.fromEntries(
              Object.keys(v)
                .sort()
                .map((k) => [k, canonical(v[k])]),
            )
        : v;
    for (const [key, value] of Object.entries(values)) {
      if (
        failed.has(source[key]) ||
        value == null ||
        value === '' ||
        value === 'unknown' ||
        value === 'unsupported' ||
        value === 'undefined' ||
        (typeof value === 'object' && !Object.keys(value).length)
      )
        continue;
      if (['platform', 'browser', 'major'].includes(key))
        out[key] = String(value).slice(0, 100);
      else {
        const hash = await crypto.subtle.digest(
          'SHA-256',
          new TextEncoder().encode(JSON.stringify(canonical(value))),
        );
        out[key] = Array.from(new Uint8Array(hash), (b) =>
          b.toString(16).padStart(2, '0'),
        ).join('');
      }
    }
    return out;
  };
}

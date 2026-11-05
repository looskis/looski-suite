import test from 'node:test';
import assert from 'node:assert/strict';
import { createVisitor, thumbmarkSignals } from './browser.mjs';
import { visitorProxy } from './server.mjs';
const location = {
  href: 'https://example.com/a?secret=1',
  origin: 'https://example.com',
  pathname: '/a',
};
test('no collection before consent; disabled fingerprinting never calls adapter; query/referrer minimized', async () => {
  const calls = [];
  let fingerprints = 0;
  const visitor = createVisitor({
    location,
    referrer: () => 'https://source.test/private?secret=1',
    collectSignals: async () => {
      fingerprints++;
      return {};
    },
    fetch: async (url, init) => {
      calls.push(init);
      return Response.json(
        init?.method === 'POST'
          ? { accepted: true }
          : { fingerprint_enabled: false },
      );
    },
  });
  assert.equal(await visitor.page(), false);
  assert.equal(calls.length, 0);
  visitor.setConsent(true);
  assert.equal(await visitor.page('/a?secret=2#fragment'), true);
  const body = JSON.parse(calls[1].body);
  assert.equal(body.page, '/a');
  assert.equal(body.referrer_host, 'source.test');
  assert.equal(fingerprints, 0);
  await visitor.withdraw();
  assert.equal(JSON.parse(calls[2].body).action, 'withdraw');
  assert.equal(await visitor.page(), false);
});
test('withdrawal interrupts a pending fingerprint and waits behind sent requests', async () => {
  let resolve;
  const pending = new Promise((r) => (resolve = r));
  const calls = [];
  const v = createVisitor({
    location,
    collectSignals: () => pending,
    fetch: async (url, init) => {
      calls.push(init);
      return Response.json({ fingerprint_enabled: true });
    },
  });
  v.setConsent(true);
  const page = v.page();
  await new Promise((r) => setImmediate(r));
  const withdrawn = v.withdraw();
  resolve({ canvas: 'x' });
  assert.equal(await page, false);
  await withdrawn;
  assert.equal(calls.filter((c) => c?.method === 'POST').length, 1);
  assert.equal(JSON.parse(calls[1].body).action, 'withdraw');
});
test('Thumbmark is lazy, disables vendor communication, omits failed and unavailable signals', async () => {
  let options;
  let calls = 0;
  const signals = thumbmarkSignals(async () => {
    calls++;
    return {
      Thumbmark: class {
        constructor(o) {
          options = o;
        }
        async get() {
          return {
            components: {
              system: {
                platform: 'MacIntel',
                browser: { name: 'Safari', version: '18.0' },
                hardwareConcurrency: 8,
              },
              canvas: { hash: 'a' },
              hardware: { videocard: 'undefined' },
              audio: { sampleHash: 55 },
            },
            error: [{ component: 'audio' }],
          };
        }
      },
    };
  });
  assert.equal(calls, 0);
  const s = await signals();
  assert.equal(options.logging, false);
  assert.equal(options.collect_beacon, false);
  assert.equal(options.api_key, '');
  assert.equal(options.simple_request, false);
  assert.equal(s.major, '18');
  assert.equal(s.gpu, undefined);
  assert.equal(s.audio, undefined);
  assert.match(s.canvas, /^[0-9a-f]{64}$/);
});
function setup() {
  const calls = [];
  let outage = false;
  const proxy = visitorProxy({
    suiteUrl: 'https://suite.example.com',
    siteKey: 'a'.repeat(22),
    origins: ['https://example.com'],
    secret: 's'.repeat(32),
    fetch: async (url, init) => {
      calls.push({ url, init });
      return Response.json(
        init.method === 'GET'
          ? { fingerprint_enabled: false, retention_days: 30 }
          : { accepted: true },
        { status: outage ? 503 : 200 },
      );
    },
  });
  return {
    proxy,
    calls,
    setOutage: () => {
      outage = true;
    },
  };
}
const req = (body, cookie, origin = 'https://example.com') =>
  new Request('https://example.com/api/visitor', {
    method: 'POST',
    headers: {
      Origin: origin,
      'Content-Type': 'application/json',
      ...(cookie ? { Cookie: cookie } : {}),
    },
    body: JSON.stringify(body),
  });
test('proxy checks origin/consent and never accepts a caller-selected token or fingerprint when disabled', async () => {
  const { proxy, calls } = setup();
  assert.equal(
    (
      await proxy.handle(
        req({ action: 'event', consent: true }, null, 'https://evil.com'),
      )
    ).status,
    403,
  );
  assert.equal(
    (await proxy.handle(req({ action: 'event', consent: false }))).status,
    400,
  );
  assert.equal(calls.length, 0);
  const r = await proxy.handle(
    req({
      action: 'event',
      consent: true,
      token: 'evil',
      event_id: crypto.randomUUID(),
      page: '/',
      components: { canvas: 'x' },
    }),
  );
  assert.equal(r.status, 200);
  const body = JSON.parse(calls[1].init.body);
  assert.match(body.token, /^[0-9a-f]{64}$/);
  assert.deepEqual(body.components, {});
  assert.match(
    r.headers.get('set-cookie'),
    /__Host-looski_visitor=.*; Path=\/; HttpOnly; SameSite=Lax; Max-Age=2592000; Secure/,
  );
  const cookie = r.headers.get('set-cookie').split(';')[0];
  assert.equal(proxy.attribution(req({}, cookie)).token, body.token);
  assert.equal(proxy.attribution(req({}, cookie.slice(0, -1) + 'Z')), null);
  assert.deepEqual(await r.json(), { accepted: true });
  const withdrawn = await proxy.handle(req({ action: 'withdraw' }, cookie));
  assert.equal(withdrawn.status, 200);
  assert.match(withdrawn.headers.get('set-cookie'), /Max-Age=0/);
  assert.equal(JSON.parse(calls[2].init.body).token, body.token);
});
test('failed withdrawal preserves the cookie so deletion can be retried', async () => {
  const { proxy, setOutage } = setup();
  const first = await proxy.handle(
    req({
      action: 'event',
      consent: true,
      page: '/',
      event_id: crypto.randomUUID(),
    }),
  );
  const cookie = first.headers.get('set-cookie').split(';')[0];
  setOutage();
  const r = await proxy.handle(req({ action: 'withdraw' }, cookie));
  assert.equal(r.status, 503);
  assert.equal(r.headers.get('set-cookie'), null);
});
test('proxy bounds request bytes and requires a strong signing secret', async () => {
  assert.throws(() =>
    visitorProxy({
      suiteUrl: 'https://suite.test',
      siteKey: 'a'.repeat(22),
      origins: ['https://example.com'],
    }),
  );
  const { proxy, calls } = setup();
  const r = await proxy.handle(
    req({ action: 'event', consent: true, page: 'x'.repeat(13000) }),
  );
  assert.equal(r.status, 413);
  assert.equal(calls.length, 0);
});

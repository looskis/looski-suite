// SPDX-License-Identifier: AGPL-3.0-only
// Call only after the host service has authenticated its bridge bearer.
export async function connectorProxy(req, registry = {}, request = fetch) {
  const incoming = new URL(req.url);
  const match = /^\/connectors\/([a-z0-9-]{1,64})(\/v1\/(?:capabilities|events|health|messages|calendar\/busy|calendar\/appointments))$/.exec(incoming.pathname);
  const failure = (status) => Response.json({ error: 'Connector unavailable' }, { status, headers: { 'Cache-Control': 'no-store' } });
  if (!match || !Object.hasOwn(registry, match[1])) return failure(404);
  const [, key, path] = match;
  const configuration = registry[key];
  const read = ['/v1/capabilities', '/v1/events', '/v1/health'].includes(path);
  if (req.method !== (read ? 'GET' : 'POST')) return failure(405);
  const base = new URL(configuration.url);
  if (base.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(base.hostname) || base.username || base.password || base.pathname !== '/' || base.search || base.hash || typeof configuration.token !== 'string' || configuration.token.length < 32) return failure(503);
  const target = new URL(path, base);
  if (path === '/v1/events') {
    const since = incoming.searchParams.get('since') || '0';
    if (!/^\d{1,18}$/.test(since)) return failure(400);
    target.searchParams.set('since', since); target.searchParams.set('limit', '50');
  }
  try {
    const chunks = []; let size = 0;
    if (!read && req.body) {
      for await (const chunk of req.body) {
        size += chunk.byteLength;
        if (size > 100000) return failure(413);
        chunks.push(Buffer.from(chunk));
      }
    }
    const response = await request(target, {
      method: req.method, redirect: 'error', signal: AbortSignal.timeout(12000),
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${configuration.token}` },
      ...(!read ? { body: Buffer.concat(chunks) } : {}),
    });
    const output = []; size = 0;
    if (response.body) for await (const chunk of response.body) {
      size += chunk.byteLength;
      if (size > 2000000) return failure(503);
      output.push(Buffer.from(chunk));
    }
    return new Response(Buffer.concat(output), { status: response.status, headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' } });
  } catch { return failure(503); }
}

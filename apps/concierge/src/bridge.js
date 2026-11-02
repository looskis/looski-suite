// SPDX-License-Identifier: AGPL-3.0-only
// Container-to-macOS adapter. The configured bearer is checked before this is
// called; only Blueski's three required endpoints are exposed, never its UI.
export async function blueskiBridge(req, config) {
  const url = new URL(req.url),
    path = url.pathname.slice('/blueski'.length);
  const allowed =
    (req.method === 'GET' && ['/healthz', '/events'].includes(path)) ||
    (req.method === 'POST' && path === '/messages');
  if (!allowed) return Response.json({ error: 'Not found' }, { status: 404 });
  const target = new URL(path, config.blueskiUrl || 'http://127.0.0.1:8788');
  if (path === '/events') {
    const since = url.searchParams.get('since') || '0';
    if (!/^\d{1,15}$/.test(since))
      return Response.json({ error: 'Invalid cursor' }, { status: 400 });
    target.searchParams.set('since', since);
    target.searchParams.set('limit', '50');
  }
  const response = await fetch(target, {
    method: req.method,
    redirect: 'error',
    signal: AbortSignal.timeout(5000),
    headers: {
      'Content-Type': 'application/json',
      ...(config.blueskiToken
        ? { Authorization: `Bearer ${config.blueskiToken}` }
        : {}),
    },
    ...(req.method === 'POST' ? { body: await req.text() } : {}),
  });
  return new Response(await response.arrayBuffer(), {
    status: response.status,
    headers: {
      'Content-Type': 'application/json',
      'Cache-Control': 'no-store',
    },
  });
}

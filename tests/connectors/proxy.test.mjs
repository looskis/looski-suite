import test from 'node:test';
import assert from 'node:assert/strict';
import { connectorProxy } from '../../apps/connectors/proxy.mjs';
const registry = { google: { url: 'http://127.0.0.1:58910', token: 'x'.repeat(40) } };
test('unknown services and paths never reach the host', async () => {
  for (const path of ['/connectors/missing/v1/health', '/connectors/google/admin', '/connectors/google/../v1/health']) {
    assert.equal((await connectorProxy(new Request('https://bridge.test'+path), registry, () => { throw Error('must not run'); })).status,404);
  }
});
test('only the connector bearer is forwarded and event queries are bounded', async () => {
  const response = await connectorProxy(new Request('https://bridge.test/connectors/google/v1/events?since=9&limit=999&token=secret', {headers:{Authorization:'Bearer bridge-secret'}}), registry, async (url, options) => {
    assert.equal(url.toString(),'http://127.0.0.1:58910/v1/events?since=9&limit=50');
    assert.equal(options.headers.Authorization,'Bearer '+'x'.repeat(40));
    assert.equal(options.redirect,'error');
    return Response.json([]);
  });
  assert.equal(response.status,200);assert.equal(response.headers.get('cache-control'),'no-store');
});
test('method and operator URL checks fail closed', async () => {
  assert.equal((await connectorProxy(new Request('https://bridge.test/connectors/google/v1/messages'), registry)).status,405);
  assert.equal((await connectorProxy(new Request('https://bridge.test/connectors/google/v1/health'), {google:{url:'http://example.com',token:'x'.repeat(40)}})).status,503);
});

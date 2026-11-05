# Visitor attribution

Visitors is an optional workspace application. Enable it in **Visitors**, create a site with exact HTTPS origins, then mount a first-party collector on that website. Collection requires recognition consent distinct from permission to reply to a lead. Fingerprinting and similarity matching default off. No visitor collection is installed on loo.ski by this backend release.

## What this slice provides

- First-party cookie visits, page paths without query strings/fragments, and referrer hostnames. No IP, full user agent, email hash, vendor identity graph or third-party beacon is stored.
- Optional browser signals and bounded exact/similar matching suggestions. A score is signal similarity, not a probability that two people are the same. Suggestions never merge visitors, copy contact attribution, or restore chat history. Staff can accept/reject/reset evidence and separately link/unlink a contact.
- An explicit `declared_contact` resolution API for trusted form/booking/concierge bridges after a contact intake. A declared email is not verified identity. The public API has no visitor/contact/history reads. Do not send attribution tokens or candidate data to an assistant prompt.
- Site settings, visitor list/detail, contact search, review, NDJSON export and permanent deletion in the UI; typed API, CLI and MCP operations. Reading visitor records requires workspace-wide `visitors.read` and `contacts.read`; managing records also requires `visitors.manage`, configuring sites requires `visitors.configure`. Owner receives these permissions; narrower grants intentionally cannot browse visitor histories.
- Postgres as the only event/identity authority in this slice, with export pagination and cascading deletion. No lossy secondary analytics store.

## First-party adapter

`apps/visitor` is MIT, framework-neutral ESM, currently private/unpublished. `visitorProxy` accepts standard Request objects and returns Response objects. Mount both GET and POST at `/api/visitor` on the website:

```js
import { visitorProxy } from '@looski/visitor/server';
const proxy = visitorProxy({
  suiteUrl: 'https://suite.example.com',
  siteKey: process.env.LOOSKI_VISITOR_SITE_KEY,
  origins: ['https://www.example.com'],
  secret: process.env.LOOSKI_VISITOR_SIGNING_SECRET, // >=32 random characters
});
export const GET = proxy.handle;
export const POST = proxy.handle;
```

The adapter uses a signed, host-only `__Host-looski_visitor` cookie with HttpOnly, Secure and SameSite=Lax. Local HTTP development uses an unprefixed cookie. The signing secret stays on the website server. Never forward a client-supplied visitor token: the adapter generates it and validates its signed cookie. Rotate the signing secret to invalidate existing cookies. Do not use the concierge session cookie for attribution.

Browser code integrates with the website's consent controls and router:

```js
import { createVisitor, thumbmarkSignals } from '@looski/visitor';
const visitor = createVisitor({
  endpoint: '/api/visitor',
  collectSignals: thumbmarkSignals(() => import('@thumbmarkjs/thumbmarkjs')),
});
// Only after the user explicitly enables recognition (or a valid saved choice):
visitor.setConsent(true);
await visitor.page();
// Call visitor.page() after future route changes while consent is active.
// On withdrawal, stop all tabs and clear the host's recognition preference:
await visitor.withdraw();
```

Install and bundle ThumbmarkJS **1.12.0** locally in the host application if using the supplied adapter. It loads lazily after consent and site configuration permit fingerprinting. Logging, collect beacons, API requests and API keys are disabled. Only normalized allowlisted components are sent; browser/platform/version remain comparable strings, other components are hashes. No CDN or remote script is needed. Without an adapter, visits remain cookie-only. This adapter API also permits a different locally bundled signal collector.

The browser library never stores consent itself. The host owns the notice, persistent choice, route hooks, cross-tab withdrawal notification and visible withdrawal control. Rejected withdrawal calls retain the cookie so deletion can be retried. Handle the error visibly; do not silently report success.

## Contact resolution

After a trusted concierge/form/booking bridge successfully resolves the submitted contact, obtain `proxy.attribution(request)` **on the website server** and send that object only to the authenticated bridge. Then call:

```text
POST /api/v1/workspaces/{workspace}/visitors/resolve
Idempotency-Key: <unique stable intake reference>
{ "site_key": "...", "token": "...", "contact_id": "...", "evidence": "declared_contact" }
```

The bridge needs the visitor/contact permissions above. A conflicting prior contact is rejected for staff review. An unavailable attribution record must not prevent the original lead capture. Attribution integration is separate from the concierge conversation key and never changes publicSession output or the agent prompt. This release does not automatically wire existing form, booking, concierge or email-click paths; that host integration remains a separate rollout.

## Retention and matching limits

Sites accept 7–180 retention days (90 default). Events expire by their creation time; visitor records expire after inactivity. The worker removes batches every 60 ticks. Shortening retention shortens existing visitor deadlines; event pruning follows on the next worker pass. Deleting contacts cascades to their attributed visitor records and events. Withdrawal and staff deletion retain only a token digest revocation for up to 180 days to block in-flight requests from recreating deleted data. No event or browser signal survives that deletion. Disabling fingerprinting purges components and match evidence, while preserving cookie-only visits. Withdrawal works even when the site/module is disabled.

Candidates are same-site, unexpired, seen within 60 days, and limited to the latest 200. Each visitor retains at most 200 reviewed/suggested candidates. Three substantial overlapping signals plus browser/platform are required. Browser family/platform changes and numeric version downgrades veto a suggestion. Missing signals do not count as agreement. Score defaults to 0.95 for optional similarity matching. These coarse signals have collisions, especially among similar devices; no automated identity decision is made.

Exports contain one visitor record followed by paginated visit records as NDJSON. They exclude expired candidates and token digests. Exporting while new events arrive is not a point-in-time snapshot; use a database recovery snapshot for that requirement. Public endpoints are throttled, including the proxy's egress address; high-volume installations should plan trusted ingress rate limiting before rollout.

## Verification

`node --test apps/visitor/visitor.test.mjs` covers consent, lazy collection, withdrawal races, data minimization, cookie signing, failed-withdrawal retry and request limits. `python3 scripts/visitors_smoke.py` clones the dedicated fixture, tests authority/isolation, matching without merges, replay/version conflicts, export paging, CLI/MCP, retention and withdrawal/deletion. Rust unit tests exercise matching vetoes, drift and missing-signal handling.

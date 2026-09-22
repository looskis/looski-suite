# Dependency and distribution review

October 13, 2026 local build review. These checks cover declared package metadata and the public OSV advisory database; they do not claim a complete container/operating-system security audit or certify a public distribution.

The initial scan found SvelteKit advisories GHSA-29g2-3rmr-qm68 (Accept-header denial of service), GHSA-866w-xmhq-wj7x and GHSA-wqjv-9729-c5q2 (remote forms), plus cookie GHSA-pxg6-pf52-xh8x. SvelteKit is now pinned to 2.70.2 and its cookie dependency overridden to compatible 0.7.2. The subsequent scan of 465 reachable public Rust/JavaScript package versions found no remaining JavaScript advisories.

Two Rust advisories remain documented:

- `atomic-polyfill` 1.0.3, RUSTSEC-2023-0089, is unmaintained. It is a lockfile dependency for other targets and is not in the active Apple Silicon dependency tree.
- `rsa` 0.9.10, RUSTSEC-2023-0071, concerns private-key RSA timing behavior. It is pulled by jsonwebtoken's crypto backend. Current identity, service and invitation tokens use HS256 with symmetric keys; the application does not perform private-key RSA operations. This is an applicability assessment, not a claim that the package is patched. Any future asymmetric-token/signing/decryption feature must reassess it before use.

Cargo metadata and reachable installed npm manifests have explicit license declarations. Rust dependencies are permissive or have a permissive alternative, with Unicode/CDLA data licenses and bundled license combinations requiring preservation. JavaScript declarations include MIT, ISC, BSD-3-Clause, Apache-2.0, Python-2.0 and MPL-2.0. No additional commercial-use restriction is introduced. Preserve component notices and preferred source as required; declared metadata alone is not a completed corresponding-source delivery mechanism.

The repository retains standard AGPLv3 text and upstream Supabase notices. [Trademark policy](../../TRADEMARKS.md) allows truthful commercial references while prohibiting misleading official affiliation. Public distribution remains gated on a corresponding-source package/access offer, full third-party notices, supported image/OS advisory review and the external-provider checks. No public release has been published by this task.

Primary advisory references: [SvelteKit Accept-header advisory](https://github.com/sveltejs/kit/security/advisories/GHSA-29g2-3rmr-qm68), [SvelteKit form pollution](https://github.com/sveltejs/kit/security/advisories/GHSA-866w-xmhq-wj7x), [SvelteKit form payload limit](https://github.com/sveltejs/kit/security/advisories/GHSA-wqjv-9729-c5q2), [cookie advisory](https://github.com/jshttp/cookie/security/advisories/GHSA-pxg6-pf52-xh8x), [Rust RSA advisory](https://rustsec.org/advisories/RUSTSEC-2023-0071.html), [atomic-polyfill advisory](https://rustsec.org/advisories/RUSTSEC-2023-0089.html). [Mozilla's MPL FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) describes component source and notice obligations; the license text remains controlling.

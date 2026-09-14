---
status: accepted
date: 2026-10-04
---

# Supabase identity with Rust application authorization

Use Supabase Auth in both self-hosted and hosted modes rather than maintaining a custom password/session implementation. Stable application user IDs map to provider identities, while Rust owns workspace membership, invitations, action permissions, and resource access.

Use pinned upstream Supabase Docker images through a validated Compose configuration and retain the standard Supabase-managed `auth` schema. Suite data and identity mappings live outside that schema. Local and hosted deployments are independent Supabase instances: each application installation uses one selected instance without synchronization or automatic failover between them. Migration is an explicit, version-validated transfer and cutover that preserves business identities and memberships; it does not promise existing session continuity.

Supabase Auth is a required foundation dependency, not an optional business application. Docker and Compose are accepted dependencies for the self-hosted distribution, and the user accepts their resource overhead. Native packaging and Apple Container were considered; neither replaces the Docker deployment for phases 0 and 1. The rationale is to reuse upstream packaging and avoid owning a custom runtime distribution to save an unmeasured amount of resources. Exact footprint and the supported Mac Docker environment still require validation.

Application business data is accessible only through approved backend and administrative paths. Under this model, do not duplicate business permissions in RLS. This avoids maintaining two diverging permission systems but gives up RLS as an additional barrier against application scoping mistakes; backend authorization, database access restrictions, scoped queries, constraints, and denial tests are therefore required across every access path.

Hosted Supabase must not expose an application Data API path that bypasses Rust. Any future use of direct Storage, Realtime, or other data access requires a separate authorization design. Local-to-hosted migration must account for identity and data migration, not only connection settings.

Support email/password and magic-link sign-in through Supabase Auth. Use configured SMTP and a stable Cloudflare Tunnel hostname for reachable application/Auth endpoints and allowlisted callbacks. Ordinary magic-link login must not implicitly create accounts; onboarding remains controlled by bootstrap and invitations. Verify SSR session establishment and expired/reused-link behavior before release.

The suite owns its permission vocabulary, roles, resource policies, and future delegated API grants. Build the identity-to-authorization boundary now so OAuth clients can be added later without rewriting business operations. Verified third-party client identity and an active workspace grant will restrict the user's existing permissions; identity alone will not authorize delegated access. OAuth server setup, client registration, consent, and grant implementation are deferred beyond phases 0 and 1. Until supported, reject delegated credentials.

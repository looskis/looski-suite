---
status: accepted
date: 2026-10-04
---

# SvelteKit frontend with server rendering

Use Svelte with SvelteKit, TypeScript, and SSR for the suite frontend. This settles the previously provisional framework choice and gives all applications one routing, rendering, and session integration model while Rust remains authoritative for business rules, permissions, and data access.

The distribution includes a JavaScript server runtime alongside Rust. Frontend server state must remain scoped to requests, and browser-facing models should come from the maintained backend contract. UI libraries, rich editors, client generation tooling, and any additional state or query libraries remain implementation decisions. Framework selection does not establish performance superiority or successful integration; phase 0 validates the chosen stack through the first CRM journey.

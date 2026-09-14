---
status: accepted
date: 2026-10-04
---

# One distribution with selectable applications

The suite must let businesses choose useful applications without operating a distributed system for each one. Start with a modular monolith in one standard distribution, with applications enabled per workspace and enforced by the backend; physically separate build bundles and runtime third-party plugins are deferred.

A small mandatory foundation supplies identity, memberships, permissions, Contacts, files, jobs, audit history, and operations. Applications retain explicit ownership and standalone workflows. Enabling an application does not grant every member access, and hiding its navigation is not enforcement. Disabling an application stops its access and workflows while preserving its data; deletion is a separate operation.

The trade-off is that the initial package contains unused application code and requires coordinated releases. In exchange, installations and cross-module transactions stay simpler. Modules are built around business operations rather than a universal custom-database platform.

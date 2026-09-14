---
status: accepted
date: 2026-10-04
---

# Shared business contacts with module-owned records

Scheduling, support, and sales need to recognize the same person even when CRM is disabled. A lightweight shared Contacts module owns workspace-scoped contacts and organizations independently of login users, while CRM owns sales records and each other module owns its own business records and rules.

Modules refer to shared identities and collaborate through small interfaces. They do not modify one another's tables. The contact timeline composes authorized views of original records; it does not become a second owner of appointments, submissions, or deals.

The shared directory is intentionally small. Sensitive employee, candidate, and application-specific details must stay in their owning modules rather than becoming broadly visible contact fields. Public intake does not prove identity or authorize replacement of trusted contact data. Matching requirements live in the [architecture](../architecture.md#shared-business-identities-and-ownership); evolving contact conflict, relationship, and merge choices live in the [CRM plan](../crm-plan.md).

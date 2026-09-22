# Contact intake checkpoint

Schema 8 adds retained source intake and explicit review through API, CLI, MCP and SSR web flows. Original input remains immutable; exact matching never overwrites trusted contact fields or silently adds unverified identifiers. Missing, invalid, conflicting, shared, archived and case-only matches remain reviewable. Authorized reviewers can link an active contact, create a corrected/provisional contact, or dismiss the input with a reason.

Automatic reuse/creation requires workspace-wide contact-read authority. Restricted readers receive the same pending outcome for hidden and unmatched identifiers; explicit visible contact IDs remain usable. Candidate lists filter before their limit and reveal no hidden counts. The shared Contacts create/edit/merge paths require explicit sharing review for new associations under restricted reads, including previously empty contacts. Retaining known associations does not require another review. Sharing permission is checked independently of whether any match exists.

Exact and case-suggestion locks serialize concurrent identity decisions without changing case-sensitive local-part keys. Version checks protect review decisions. Source evidence, contact creation, audit and request replay commit in one transaction. Runtime grants disallow updates to original source evidence.

Validated on the isolated upgraded installation with `make check test`, clippy, and the sequential `make acceptance` suite: identity, Contacts, Organizations, Tags and Intake. Intake tests cover same-identifier and case-variant races, non-disclosing restricted outcomes, mixed exact/case-only inputs, stale reviews, audit failure rollback, immutable evidence and representative browser/CLI/MCP workflows. The complete migration chain, new-owner grants and all implemented journeys also passed on the fresh schema 9 foundation rehearsal.

This is the shared identity-resolution operation. Forms, Scheduling and their durable configured follow-up workflows are separate delivery increments.

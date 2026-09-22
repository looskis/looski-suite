# Contact intake implementation plan

This implements the approved matching table in the CRM plan. It is not yet shipped. It supports agent scripts now and later Forms/Scheduling through the same Contacts boundary; public submission authorization remains a separate feature.

## Operations and retained evidence

An authenticated intake operation accepts bounded source context, proposed person fields and identifiers, plus an optional explicit contact ID. Store the original input in Contacts-owned intake data before returning a resolved association or a pending review. The input is not copied into diagnostic logs or authoritative audit details. An intake has its own stable ID, owner membership, version and workspace-qualified optional contact reference.

Identifiers use the existing normalization and locking implementation. Invalid identifiers never enter the registry. Preserve malformed/missing identity input for review; never infer identity from a name. An unambiguous permitted, active contact can be reused without overwriting its fields or adding unverified identifiers. A request with valid unmatched identifiers and a usable name can create one contact when the actor also has create authority. Exact identifier locks plus an ordered case-suggestion lock namespace cover this decision and creation, preventing different request keys from creating two people for the same identifiers.

Conflicting matches, shared identifiers, archived matches, case-only email suggestions, invalid input, insufficient create authority and hidden candidates go to review. A submitter cannot discover hidden contacts through response fields, counts or review reasons. Actors lacking workspace-wide contact-read scope retain all non-explicit intake for review, so unmatched and hidden identifiers produce the same result. Explicit Contacts creation with identifiers and restricted visibility likewise requires deliberate sharing review, whether or not a hidden match exists. Review projections show only contact candidates permitted by current contact-read scope. The original input is visible only through the scoped intake-read operation.

A reviewer can associate a permitted active contact, dismiss the intake with a reason, or create a deliberately separate/provisional contact using validated corrected input. Shared-identifier creation requires the existing explicit sharing permission. No review action merges people implicitly. Linking does not authenticate the person or grant workspace access.

## Reliability and interfaces

Submission and resolution use bounded idempotency keys, current authority checks and expected versions. Contact changes, intake state, audit and replay results share one PostgreSQL transaction. Original input remains immutable after resolution; later contact edits or merges do not rewrite its source evidence. Old contact references use the existing alias policy. API/OpenAPI, CLI, MCP and a browser review queue expose the same operations.

Tests must demonstrate unambiguous reuse, no trusted-field overwrite, invalid/missing identity preservation, explicit review, hidden-candidate denial, case-sensitive matching, archived matches, same/different-key concurrency, stale review, original evidence, rollback and interface parity.

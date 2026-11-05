# Quotes and invoicing

Design target, not a shipped billing application. The first workflow replaces preparing a quote, accepting its exact version, issuing a numbered invoice, and reconciling manually recorded payments. Payment collection and accounting remain replaceable external connections.

## Records and invariants

A workspace owns legal entities, customers, catalog items, quote revisions, acceptances, invoices, credit notes, payment allocations and recurring agreements. A customer can link to a Contact or Organization without requiring CRM. Every issued document snapshots the seller/customer details, currency, tax settings, terms and line calculations; later catalog or address edits cannot rewrite it. A line stores decimal quantity, unit, decimal unit price, discount and tax calculation with an explicit rounding policy. Never use binary floating point for money. Currency precision is defined per currency; exchange rates have a source/date and are frozen for reporting.

Quotes move from draft to issued, accepted, declined, expired or superseded. Acceptance references a specific immutable revision and records who accepted, when and how. Reissuing terms produces a new revision and invalidates the previous acceptance link. Deposits, options and milestone schedules are explicit, not hidden negative lines. Approval thresholds can gate issuance.

Invoices move from draft to issued, partially paid, paid or voided. Overdue is derived from remaining balance and due date. Issuance allocates a unique number in a transaction scoped to legal entity and numbering series. Issued financial content is immutable; corrections use linked credit notes and replacement documents. A payment is independent of its allocations: support partial payments, overpayments, reversals and multi-invoice allocation without allowing an allocated amount to exceed available funds. Webhooks and imports deduplicate by provider identity plus event ID; replay never records another payment.

Recurring agreements generate draft invoices by unique agreement/period keys. Pause, cancellation, proration and future price changes have explicit effective dates. Time and expenses become invoice lines only after review. A source allocation ledger prevents billing the same approved time twice, supports partial quantities, and releases allocations correctly on draft deletion/credit correction.

## Looski's first real scenario

Easy Street Offers has a $5,000 monthly subscription and $400/hour work, expected at two hours a week. Keep these as separate agreement lines. Draft the subscription once per monthly period and pull actual approved billable time for the hourly line; do not invoice an estimated annual $41,600 as a fixed commitment. Track unbilled time, billed hours, recurring revenue and cash collected separately. The four open $360,000/year opportunities can produce quotes with annual or monthly billing without treating pipeline value as recognized revenue.

## Ideal experience and extensibility

Provide reusable branded templates, accessible HTML and reproducible PDF, expiring customer review/acceptance links, delivery status, reminders, statement of account, deposits, milestones, recurring plans, multiple currencies, tax-inclusive/exclusive prices and credit notes. A customer portal exposes only deliberately shared documents. Draft previews and send actions are distinct. AI can suggest descriptions or classifications; issuance, changes to amounts and sending remain explicit staff actions.

Use shared Files, Contacts, Organizations, Tasks, Time, CRM, permissions, audit, durable jobs and API/CLI/MCP operations. Provider adapters handle payments, accounting exports and delivery. No payment processor IDs in core lifecycle rules. Signed payment events update a reconciliation queue before they affect allocations; verify amount/currency/customer. External accounting sync records versions, external IDs and conflicts rather than overwriting issued documents. Connect inventory reservations/fulfillment later through explicit references.

## Delivery sequence and acceptance

1. Catalog, quotes, immutable revisions and recorded acceptance; customer snapshots, decimal calculations, HTML/PDF, audit and interface parity.
2. Numbered invoices, credit notes, payment allocations, aging and exports. Concurrent issuance cannot duplicate a number; retries cannot duplicate documents/payments; crediting and reversals preserve arithmetic.
3. Agreements and reviewed time/expense billing. Validate an ESO month end to end, including timezone boundaries, duplicate generation retries and corrected timesheets.
4. Optional payment/accounting adapters, customer portal, approval policies and automated reminders. Revoke access, replay webhooks, simulate ambiguous delivery and restore a backup without resending invoices or charging anyone.

Before production issuance, choose legal entity details, numbering series, currencies, payment terms, tax configuration and accounting export target. These are configuration choices to validate with the operator; no jurisdiction-specific tax engine or filing promise is part of the first release.

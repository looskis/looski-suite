# Inventory and purchasing

Design target, not a shipped inventory application. Start with accurate stock and purchase receiving; add richer warehouse operations after the transactional ledger is proven. Individually assigned equipment belongs to Assets, with optional links to inventory serials.

## Authority and invariants

A workspace owns items/SKUs, variants, unit definitions, locations/bins, suppliers, purchase orders, receipts, reservations, shipments, returns and an append-only movement ledger. Quantities use exact decimals and declared units; item conversions must be explicit, versioned and physically valid. Keep stock on hand, reserved, available, incoming and quarantined separate. A count is never a mutable number disconnected from its movements.

Each posted movement has item, quantity, unit, source/destination, effective time, recorded time, actor, reason and source document. Corrections use reversing movements. Transfers debit and credit atomically; no stock disappears between locations. Reservations serialize per item/location and cannot exceed available stock unless a workspace deliberately enables backorders. Fulfillment consumes a reservation and stock in one transaction. Durable idempotency keys prevent duplicate receiving/shipping after retries or barcode double scans.

Purchase orders move draft → approved → issued → partially received → received/closed/cancelled. Approval freezes the authorized revision. Receiving checks remaining ordered quantities and explicit over-receipt tolerances. Support partial receipts, damaged/quarantined goods, supplier returns and landed-cost adjustments without rewriting historical receipts. A supplier invoice can match PO/receipt quantities and prices for review; it does not automatically authorize a payment.

## Ideal experience

Searchable item catalog, barcode labels/scanning, multiple locations, stock counts with variance review, replenishment suggestions, supplier lead times, min/max levels, purchase approval, partial receiving, picking/packing, returns and traceability. Add lot/serial tracking, expiry and recall paths, kits/bundles, assembly/disassembly and unit conversions as separate slices. Offline scanning queues must detect version conflicts and require review rather than claiming offline reservations are guaranteed.

Decouple physical quantity from valuation. Initially export movements and purchase costs; an optional valuation module can implement a chosen weighted-average or FIFO policy with immutable cost layers, explicit backdating rules and reconciled opening balances. Never label an approximate stock-cost report a complete accounting ledger. Provider adapters support purchasing channels, shipping labels, commerce and accounting; core stock rules do not depend on one vendor.

## Delivery sequence and acceptance

1. Items, units, locations, opening balances, receipts/issues/transfers, movement history and CSV import/export. Prove concurrent movements conserve stock and cross-workspace references fail.
2. Reservations, fulfillment, counts and corrections. Race two reservations for the last unit; exactly one succeeds. Retry transfers and count postings; balances must remain unchanged. Validate negative-stock policy and backdated corrections explicitly.
3. Suppliers, PO revisions, approvals, partial receiving and returns. Reject accidental over-receipt; retain source links and expected-version checks across every channel.
4. Lot/serial/expiry, replenishment, three-way match, valuation and external adapters. Test backup restoration, replay, unit conversion rounding, serialized uniqueness and complete recall tracing.

Before importing stock, choose base units, locations, negative-stock policy, tracking requirements and opening-balance date. Invoices can reference shipments and purchase invoices can reference receipts, but inventory remains useful with CRM, invoicing and accounting disabled. Use shared permissions, audit, Jobs, Files, approvals and interface parity. Staff adjustments always require a reason.

# Expenses and approvals

Design target, not a shipped expense application. Replace receipt collection, review, policy decisions and an auditable accounting handoff. Reimbursement/payment rails, payroll and tax filing remain external.

## Domain and workflow

An expense has an owner, merchant, transaction date, original decimal amount/currency, category, cost center/project, business purpose, receipt files and optional billable-customer link. Reports group expenses without losing per-item decisions. Support split allocations, mileage/per-diem policies, employee-paid versus company-paid purchases and credits/refunds. Exchange rates, policy versions and calculated home-currency amounts are frozen on submission and disclosed in review.

Draft → submitted → in review → approved/rejected/returned → exported → reimbursed/reconciled. Returning an expense creates an editable revision; resubmission invalidates obsolete decisions. Exported financial facts cannot be silently edited. Approved does not mean paid; reimbursement confirmation comes from a verified import or an authorized manual entry. Every transition records actor, timestamp, reasons and expected version.

A versioned approval policy selects steps by amount, category, team, project and legal entity. Sequential and parallel steps have explicit all/any quorum rules. Prevent self-approval, including delegation loops and role changes; route the exception to an independent approver. Delegation has a time window and audit trail. Escalation sends a task/notification without implicitly approving. Resolve the approver chain at submission, retain its policy version, and revalidate current authority at decision time. Policy changes require deliberate reassessment of in-flight reports.

## Ideal experience

Mobile receipt capture, forwarded receipt intake, draft OCR extraction, duplicate suggestions, split lines, policy explanations, manager queue, bulk decisions with per-item outcomes, reminders and employee status. OCR and AI results remain suggestions until reviewed. Receipts stay private Files; public share URLs are never a default. Duplicate checks combine receipt hash, merchant/date/amount and import IDs, with an explicit override reason for legitimate duplicates.

Card and bank feeds are adapters with replayable cursors and reconciliation status. An expense must not be recreated on import retry. Match card transactions to submitted receipts without claiming that a fuzzy match is certain. Accounting export includes a stable external reference, chart-of-accounts mapping and frozen totals; delivery acknowledgment is separate from financial acceptance. Time/CRM/invoicing links are optional, and approval grants do not reveal unrelated employee records.

## Delivery sequence and acceptance

1. Manual expense/receipt entry, private ownership, reports, categories, export and audit. Test permission isolation, multi-currency precision, split totals and duplicate import prevention.
2. Versioned policy evaluation and approval queue. Test simultaneous decisions, threshold boundaries, self-approval, delegation cycles, removed approvers, returned reports and policy changes.
3. OCR review, reconciliation feeds and billable expense handoff. Prevent a receipt or expense allocation from being invoiced twice; preserve the original transaction through refunds.
4. Travel policies, purchase preapproval, budget monitoring and accounting adapters. Recovery must not repeat reimbursements or exports; an unavailable adapter leaves a visible pending state.

Shared approval machinery should emerge from this concrete workflow and quote/purchase-order issuance, then become a small reusable module: subjects, policy revisions, decisions, delegations and an event log. It must not become a general workflow-builder prerequisite. API, web, CLI and MCP must execute identical authority and transition checks.

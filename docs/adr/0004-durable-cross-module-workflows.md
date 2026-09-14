---
status: accepted
date: 2026-10-04
---

# Durable workflows for cross-module follow-up

A confirmed appointment must survive a temporary failure in CRM follow-up. Commit primary business records and durable scheduling of follow-up work atomically in PostgreSQL, then execute built-in recipes through module interfaces with retries, recorded progress, authorization, and idempotency.

Scheduling with a custom intake form commits the appointment, submission, association, and booking event together. The confirmed booking is the single trigger for its CRM intake recipe; the embedded submission does not independently trigger the same ingestion. This is durable event delivery for follow-up work, not a decision to event-source all business state.

The trade-off is that connected views may briefly lag behind a successful booking and operators need visible workflow status and recovery. The benefit is a clear primary outcome and recoverable secondary work. At-least-once execution is assumed; external effects require provider idempotency where available or an explicit reconciliation path for uncertain outcomes. A general visual workflow builder is deferred until concrete recipes establish its requirements.

PGMQ is the selected initial queue implementation, in the same PostgreSQL database as business records. Enqueue through the business SQLx transaction and verify commit/rollback behavior. Use persistent logged queues, application workers, leases, bounded retries, and recorded step completion. Queue transport does not remove the requirement for idempotent business operations. Validate the extension in both local and hosted database configurations. RabbitMQ and BullMQ remain possible future migrations if needed; an external broker would require revisiting atomic scheduling, normally through an outbox.

# Suite database migrations

Put suite-owned PostgreSQL migrations in `migrations/<timestamp>_<description>.sql` and apply them with the pinned Supabase CLI to an explicitly selected database. SQLx is used for runtime data access, not as a second migration runner.

Supabase manages its own `auth` schema and bootstrap/upgrade scripts. Suite tables and migrations stay outside `auth`. Development seeds are separate from production migrations. ClickHouse setup is separate from this PostgreSQL history.

See the [setup contract](../docs/setup-contract.md) for migration targeting, Compose naming, service readiness, and proposed ports. No CLI configuration, SQL migrations, or deployment has been initialized yet.

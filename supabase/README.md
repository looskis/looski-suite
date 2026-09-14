# Supabase integration

`config.toml` identifies CLI migration workflows; `compose.yaml` owns the running local stack. Do not run `supabase start` alongside the suite Compose deployment.

Suite SQL lives in `migrations/<timestamp>_<description>.sql`. `make up` runs a fresh container with Supabase CLI 2.75.0 and an explicit private database target before releasing the API. SQLx does not run migrations. Create a new migration with the pinned CLI’s `supabase migration new <name>` command; never edit an applied migration. Migration-content drift enforcement and production upgrades remain pending.

The first migration enables the image’s bundled PGMQ extension and creates `suite.schema_version`. There are no Contacts/CRM tables yet. The separate runtime login cannot administer the suite schema. Supabase’s image and Auth service own upstream roles, the standard `auth` schema, and its migrations; suite migrations must not change them.

See the [setup contract](../docs/setup-contract.md), [scaffold evidence](../docs/scaffold-status.md), and [development commands](../README.md).

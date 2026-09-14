#!/bin/sh
set -eu
# Serialize jobs sharing this installation's lock volume. Copy the read-only
# migration source because the CLI may create its local scratch directory.
exec 9>/locks/migration.lock
flock -x 9
mkdir -p /tmp/migrations-work
cp -R /work/supabase /tmp/migrations-work/
cd /tmp/migrations-work
supabase migration up --db-url "$(cat /run/secrets/migration_url)" --yes

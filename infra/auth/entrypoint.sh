#!/bin/sh
set -eu
export GOTRUE_DB_DATABASE_URL="postgres://supabase_auth_admin:$(cat /run/secrets/db_password)@db:5432/postgres"
export GOTRUE_JWT_SECRET="$(cat /run/secrets/jwt_secret)"
exec su-exec supabase auth

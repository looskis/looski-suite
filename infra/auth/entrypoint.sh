#!/bin/sh
set -eu
export GOTRUE_DB_DATABASE_URL="postgres://supabase_auth_admin:$(cat /run/secrets/db_password)@db:5432/postgres"
export GOTRUE_JWT_SECRET="$(cat /run/secrets/jwt_secret)"
export GOTRUE_SMTP_HOST="$(jq -er .host /run/secrets/auth_mail)"
export GOTRUE_SMTP_PORT="$(jq -er .port /run/secrets/auth_mail)"
export GOTRUE_SMTP_USER="$(jq -r .user /run/secrets/auth_mail)"
export GOTRUE_SMTP_PASS="$(jq -r .password /run/secrets/auth_mail)"
export GOTRUE_SMTP_ADMIN_EMAIL="$(jq -er .from /run/secrets/auth_mail)"
export GOTRUE_SMTP_SENDER_NAME="$(jq -er .sender_name /run/secrets/auth_mail)"
if [ "${GOTRUE_EXTERNAL_GOOGLE_ENABLED:-false}" = "true" ]; then
  export GOTRUE_EXTERNAL_GOOGLE_CLIENT_ID="$(jq -er .client_id /run/secrets/auth_google)"
  export GOTRUE_EXTERNAL_GOOGLE_SECRET="$(jq -er .client_secret /run/secrets/auth_google)"
fi
exec su-exec supabase auth

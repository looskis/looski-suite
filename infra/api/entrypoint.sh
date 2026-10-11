#!/bin/sh
set -eu
# File-backed Compose secrets retain host UID/mode on Linux. Root performs only
# this handoff into ephemeral storage; the API itself runs as UID/GID 10001.
if [ "${1:-}" = api ] || [ "${1:-}" = bootstrap-owner ] || [ "${1:-}" = worker ]; then
  install -D -o 10001 -g 10001 -m 0400 "$LOOSKI_DATABASE_URL_FILE" /run/looski/database_url
  export LOOSKI_DATABASE_URL_FILE=/run/looski/database_url
fi
if [ "${1:-}" = api ] || [ "${1:-}" = bootstrap-owner ] || [ "${1:-}" = worker ]; then
  install -D -o 10001 -g 10001 -m 0400 "$LOOSKI_JWT_SECRET_FILE" /run/looski/jwt_secret
  export LOOSKI_JWT_SECRET_FILE=/run/looski/jwt_secret
fi
if [ "${1:-}" = api ] || [ "${1:-}" = worker ]; then
  install -D -o 10001 -g 10001 -m 0400 "$LOOSKI_INVITATION_KEY_FILE" /run/looski/invitation_key
  export LOOSKI_INVITATION_KEY_FILE=/run/looski/invitation_key
fi
if [ "${1:-}" = worker ] && [ -r "${LOOSKI_SMTP_CONFIG_FILE:-/nonexistent}" ]; then
  install -D -o 10001 -g 10001 -m 0400 "$LOOSKI_SMTP_CONFIG_FILE" /run/looski/smtp_config
  export LOOSKI_SMTP_CONFIG_FILE=/run/looski/smtp_config
fi
if [ "${LOOSKI_DIAGNOSTICS_MODE:-disabled}" = local ]; then
  if [ -r "${LOOSKI_DIAGNOSTICS_KEY_FILE:-/nonexistent}" ]; then
    install -D -o 10001 -g 10001 -m 0400 "$LOOSKI_DIAGNOSTICS_KEY_FILE" /run/looski/diagnostics_key
    export LOOSKI_DIAGNOSTICS_KEY_FILE=/run/looski/diagnostics_key
  fi
fi
exec setpriv --reuid=10001 --regid=10001 --clear-groups --no-new-privs looski "$@"

#!/bin/sh
set -eu
# File-backed Compose secrets retain host UID/mode on Linux. Root performs only
# this handoff into ephemeral storage; the API itself runs as UID/GID 10001.
if [ "${1:-}" = api ] || [ "${1:-}" = bootstrap-owner ]; then
  install -D -o 10001 -g 10001 -m 0400 "$LOOSKI_DATABASE_URL_FILE" /run/looski/database_url
  export LOOSKI_DATABASE_URL_FILE=/run/looski/database_url
  install -D -o 10001 -g 10001 -m 0400 "$LOOSKI_JWT_SECRET_FILE" /run/looski/jwt_secret
  export LOOSKI_JWT_SECRET_FILE=/run/looski/jwt_secret
fi
exec setpriv --reuid=10001 --regid=10001 --clear-groups --no-new-privs looski "$@"

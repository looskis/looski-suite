#!/bin/sh
set -eu
export POSTGRES_PASSWORD="$(cat /run/secrets/db_password)"
export LOOSKI_RUNTIME_PASSWORD="$(cat /run/secrets/runtime_password)"
exec docker-entrypoint.sh "$@"

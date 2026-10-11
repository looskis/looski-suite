#!/bin/sh
set -eu
# Rebuild and restart the requested command whenever the Rust sources change.
# API and worker share one target volume; Cargo's build lock serializes them.
# File events from macOS VM bind mounts are unreliable, so poll instead.
# A failed start retries, so the worker recovers once migrations are applied
# without waiting for another source change.
exec watchexec --poll 500ms --restart --shell=none \
  --watch crates --watch Cargo.toml --watch Cargo.lock \
  -- /bin/sh -c 'cargo build --locked -p looski && while :; do
    /bin/sh /usr/local/bin/looski-entrypoint "$@" || echo "looski $1 exited with $?; retrying in 5s (make dev-migrate applies new migrations)"
    sleep 5
  done' looski-dev "$@"

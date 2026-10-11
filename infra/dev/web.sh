#!/bin/sh
set -eu
# Dependencies live in container volumes; the host checkout keeps its own.
pnpm install --frozen-lockfile --store-dir /pnpm-store
if [ "$LOOSKI_DEV_WEB_SCHEME" = https ]; then
  # Vite must see the browser's HTTPS scheme so SvelteKit derives the public
  # origin for form-origin checks and Secure cookies. Only the gateway, on the
  # private Compose network, connects with this throwaway certificate.
  mkdir -p "$LOOSKI_DEV_TLS_DIR"
  openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 30 -subj /CN=web \
    -keyout "$LOOSKI_DEV_TLS_DIR/key.pem" -out "$LOOSKI_DEV_TLS_DIR/cert.pem" 2>/dev/null
fi
exec pnpm --filter @looski/web exec vite dev --host 0.0.0.0 --port 3000 --strictPort

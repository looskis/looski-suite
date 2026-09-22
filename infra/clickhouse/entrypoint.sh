#!/bin/sh
set -eu
key_hash=$(tr -d '\n' < /run/secrets/diagnostics_key | sha256sum | cut -d ' ' -f 1)
cat > /etc/clickhouse-server/users.d/looski.xml <<EOF
<clickhouse><users>
 <default><networks replace="replace"><ip>127.0.0.1</ip><ip>::1</ip></networks></default>
 <looski_ingest>
  <password_sha256_hex>${key_hash}</password_sha256_hex>
  <networks><ip>::/0</ip></networks>
  <grants><query>GRANT INSERT ON looski_diagnostics.events</query></grants>
 </looski_ingest>
</users></clickhouse>
EOF
chmod 0644 /etc/clickhouse-server/users.d/looski.xml
exec /entrypoint.sh "$@"

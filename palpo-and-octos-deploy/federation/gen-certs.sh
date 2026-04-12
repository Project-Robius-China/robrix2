#!/usr/bin/env bash
# ============================================================
# Generate self-signed TLS certificates for local federation
# ============================================================
# Creates certs/node1.{crt,key} and certs/node2.{crt,key}.
# CN and subjectAltName match Docker network aliases used as server_name.
#
# These certs are ONLY suitable for local testing.
# Production requires certificates from a trusted CA (e.g., Let's Encrypt).
# ============================================================

set -euo pipefail

CERT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/certs"
mkdir -p "$CERT_DIR"

for NODE in 1 2; do
  HOSTNAME="palpo-${NODE}"
  CRT="${CERT_DIR}/node${NODE}.crt"
  KEY="${CERT_DIR}/node${NODE}.key"

  if [[ -f "$CRT" && -f "$KEY" ]]; then
    echo "[skip] $CRT already exists (delete to regenerate)"
    continue
  fi

  echo "[gen]  node${NODE}: CN=${HOSTNAME}"
  openssl req -x509 -nodes -newkey rsa:2048 -days 365 \
    -keyout "$KEY" \
    -out "$CRT" \
    -subj "/CN=${HOSTNAME}" \
    -addext "subjectAltName=DNS:${HOSTNAME}" \
    2>/dev/null
done

echo
echo "Done. Certificates written to $CERT_DIR/"
ls -la "$CERT_DIR"/

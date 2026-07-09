#!/usr/bin/env bash
# ============================================================================
# Run your personal Octos agent in Matrix user-account (Direct) mode.
#
# Usage:
#   1. cp myagent.example.json myagent.json   # then edit the 4 values marked below
#   2. cp .env.example .env                    # then put your real DEEPSEEK_API_KEY in it
#   3. ./start.sh
#
# The agent runs in the foreground; watch the log for
#   "Matrix user channel authenticated"
# then Ctrl-C to stop. See ../01-connecting-your-own-octos-to-robrix.md for details.
# ============================================================================
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"

# --- 1. octos binary -------------------------------------------------------
# Must be a build that includes the Matrix user-account channel.
# Put `octos` on your PATH, or export OCTOS_BIN=/full/path/to/octos before running.
OCTOS_BIN="${OCTOS_BIN:-octos}"
command -v "$OCTOS_BIN" >/dev/null 2>&1 \
  || { echo "ERROR: octos binary not found ('$OCTOS_BIN'). Set OCTOS_BIN=/path/to/octos." >&2; exit 1; }

# --- 2. profile + secrets --------------------------------------------------
PROFILE="$DIR/myagent.json"
[ -f "$PROFILE" ] \
  || { echo "ERROR: $PROFILE not found. Run: cp myagent.example.json myagent.json  (then edit it)" >&2; exit 1; }

[ -f "$DIR/.env" ] \
  || { echo "ERROR: $DIR/.env not found. Run: cp .env.example .env  (then add your key)" >&2; exit 1; }
set -a; . "$DIR/.env"; set +a
[ -n "${DEEPSEEK_API_KEY:-}" ] \
  || { echo "ERROR: DEEPSEEK_API_KEY is empty in .env" >&2; exit 1; }

# --- 3. proxy guard (the #1 reason a native octos never replies) -----------
# If your shell has a global proxy (Clash etc.), octos would route the
# homeserver /sync through it too and fail with 502 Bad Gateway. Exclude ONLY
# your homeserver host so external calls (DeepSeek) still use the proxy.
#
# Replace the host below with YOUR homeserver's host/IP (the part after the
# colon in server_name). For a public HTTPS homeserver you usually do not need
# a proxy at all and can leave this as-is.
export NO_PROXY="127.0.0.1,localhost,matrix.example.org"
export no_proxy="$NO_PROXY"

# --- 4. launch -------------------------------------------------------------
echo "starting personal octos (Direct mode) — Ctrl-C to stop"
exec "$OCTOS_BIN" gateway \
  --profile "$PROFILE" \
  --data-dir "$DIR/octos-data" \
  --provider deepseek --model deepseek-chat

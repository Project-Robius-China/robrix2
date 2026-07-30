#!/usr/bin/env bash
set -euo pipefail

DEMO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
STATE_ROOT=""
CONFIRMED=0
DRY_RUN=0

usage() {
  echo "usage: $0 --state-root PATH --confirm [--dry-run]" >&2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --state-root)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      STATE_ROOT="$2"
      shift 2
      ;;
    --confirm)
      CONFIRMED=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

[ "$CONFIRMED" -eq 1 ] || { echo "[demo-reset] --confirm is required" >&2; exit 2; }
[ -n "$STATE_ROOT" ] || { echo "[demo-reset] --state-root must not be empty" >&2; exit 2; }
[ -d "$STATE_ROOT" ] || { echo "[demo-reset] state root must be an existing directory" >&2; exit 2; }

mkdir -p "$DEMO_DIR/.runtime"
ALLOWED_ROOT="$(cd "$DEMO_DIR/.runtime" && pwd -P)"
STATE_ROOT="$(cd "$STATE_ROOT" && pwd -P)"
case "$STATE_ROOT" in
  "$ALLOWED_ROOT"|"$ALLOWED_ROOT"/*) ;;
  *) echo "[demo-reset] state root must be inside $ALLOWED_ROOT" >&2; exit 2 ;;
esac

CONFIGURED_RUNTIME="${PALPO_RUNTIME_DIR:-./.runtime}"
case "$CONFIGURED_RUNTIME" in
  /*) ;;
  *) CONFIGURED_RUNTIME="$DEMO_DIR/$CONFIGURED_RUNTIME" ;;
esac
[ -d "$CONFIGURED_RUNTIME" ] || {
  echo "[demo-reset] PALPO_RUNTIME_DIR must be an existing directory" >&2
  exit 2
}
CONFIGURED_RUNTIME="$(cd "$CONFIGURED_RUNTIME" && pwd -P)"
[ "$STATE_ROOT" = "$CONFIGURED_RUNTIME" ] || {
  echo "[demo-reset] --state-root must match PALPO_RUNTIME_DIR ($CONFIGURED_RUNTIME)" >&2
  exit 2
}

COMPOSE_FILE="$DEMO_DIR/compose.yml"
ENV_FILE="${PALPO_ENV_FILE:-$DEMO_DIR/.env}"
PROJECT_ARGS=()
if [ -n "${PALPO_COMPOSE_PROJECT_NAME:-}" ]; then
  PROJECT_ARGS=(--project-name "$PALPO_COMPOSE_PROJECT_NAME")
fi

if [ "$DRY_RUN" -eq 1 ]; then
  echo "DRY-RUN docker compose ${PROJECT_ARGS[*]} --env-file $ENV_FILE -f $COMPOSE_FILE --profile palpo-local down --remove-orphans"
  echo "DRY-RUN rm -rf -- $STATE_ROOT/config $STATE_ROOT/state"
  exit 0
fi

docker compose "${PROJECT_ARGS[@]}" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" --profile palpo-local down --remove-orphans
rm -rf -- "$STATE_ROOT/config" "$STATE_ROOT/state"
echo "[demo-reset] removed generated config and state below $STATE_ROOT"

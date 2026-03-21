#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${POIESISD_URL:-http://localhost:8080}"
TES_API="$BASE_URL/ga4gh/tes/v1"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; exit 1; }

# ── 1. Submit a simple task ──────────────────────────────────────────────────
info "Submitting task: sleep 10 then echo hello"
RESPONSE=$(curl -sf -X POST "$TES_API/tasks" \
  -H 'Content-Type: application/json' \
  -d '{
    "executors": [
      {
        "image": "alpine:latest",
        "command": ["sh", "-c", "sleep 10 && echo hello from poiesisd"]
      }
    ]
  }') || fail "Failed to submit task"

TASK_ID=$(echo "$RESPONSE" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
[ -z "$TASK_ID" ] && fail "No task ID in response: $RESPONSE"
info "Task created: $TASK_ID"

# ── 2. Poll GET /tasks/{id} until terminal state ────────────────────────────
info "Polling task state via API (timeout 120s)..."
SECONDS=0
TIMEOUT=120
STATE=""

while true; do
  if [ $SECONDS -ge $TIMEOUT ]; then
    fail "Timed out after ${TIMEOUT}s. Last state: $STATE"
  fi

  sleep 2

  TASK_RESP=$(curl -sf "$TES_API/tasks/$TASK_ID" 2>/dev/null || true)
  if [ -n "$TASK_RESP" ]; then
    STATE=$(echo "$TASK_RESP" | sed -n 's/.*"state":"\([^"]*\)".*/\1/p')
  fi

  info "  state: ${STATE:-<unknown>}"

  case "$STATE" in
    COMPLETE)
      info "Task completed successfully!"
      break
      ;;
    EXECUTOR_ERROR|SYSTEM_ERROR|CANCELED)
      fail "Task ended with state: $STATE"
      ;;
    QUEUED|INITIALIZING|RUNNING|"")
      ;;
    *)
      warn "Unexpected state: $STATE"
      ;;
  esac
done

info "All checks passed!"

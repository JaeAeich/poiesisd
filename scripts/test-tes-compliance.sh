#!/usr/bin/env bash
#
# TES compliance tests against the GA4GH TES v1.1.0 OpenAPI spec.
# Exercises: create, get, state transitions, inputs (url + content),
# outputs, volumes, env, workdir, stdin/stdout/stderr redirect,
# ignore_error, executor error, and multi-executor logging.
#
set -euo pipefail

BASE_URL="${POIESISD_URL:-http://localhost:8080}"
TES_API="$BASE_URL/ga4gh/tes/v1"

S3_ENDPOINT="${S3_ENDPOINT:-http://localhost:9000}"
S3_BUCKET="${S3_BUCKET:-data}"
S3_ACCESS_KEY="${S3_ACCESS_KEY_ID:-admin}"
S3_SECRET_KEY="${S3_SECRET_ACCESS_KEY:-adminadmin}"

export AWS_ACCESS_KEY_ID="$S3_ACCESS_KEY"
export AWS_SECRET_ACCESS_KEY="$S3_SECRET_KEY"
export AWS_DEFAULT_REGION="us-east-1"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

PASSED=0
FAILED=0
ERRORS=()

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
step()  { echo -e "${CYAN}[TEST]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; FAILED=$((FAILED+1)); ERRORS+=("$*"); }
pass()  { echo -e "${GREEN}[PASS]${NC}  $*"; PASSED=$((PASSED+1)); }

s3() { aws --endpoint-url "$S3_ENDPOINT" s3 "$@"; }
s3api() { aws --endpoint-url "$S3_ENDPOINT" s3api "$@"; }

# Poll task until terminal state. Sets STATE global.
# Usage: poll_task <task_id> [timeout_seconds]
poll_task() {
  local task_id="$1"
  local timeout="${2:-120}"
  STATE=""
  SECONDS=0
  while true; do
    if [ $SECONDS -ge "$timeout" ]; then
      STATE="TIMEOUT"
      return 1
    fi
    sleep 2
    local resp
    resp=$(curl -sf "$TES_API/tasks/$task_id" 2>/dev/null || true)
    if [ -n "$resp" ]; then
      STATE=$(echo "$resp" | sed -n 's/.*"state":"\([^"]*\)".*/\1/p')
    fi
    case "$STATE" in
      COMPLETE|EXECUTOR_ERROR|SYSTEM_ERROR|CANCELED) return 0 ;;
      QUEUED|INITIALIZING|RUNNING|"") ;;
      *) return 0 ;;
    esac
  done
}

# Submit a task and return task ID. Sets TASK_ID global.
# Usage: submit_task '<json>'
submit_task() {
  local json="$1"
  local resp
  resp=$(curl -sf -X POST "$TES_API/tasks" \
    -H 'Content-Type: application/json' \
    -d "$json") || { fail "Failed to submit task"; return 1; }
  TASK_ID=$(echo "$resp" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
  [ -z "$TASK_ID" ] && { fail "No task ID in response: $resp"; return 1; }
  return 0
}

echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════${NC}"
echo -e "${BOLD}       TES v1.1.0 Compliance Test Suite            ${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════${NC}"
echo ""

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 1: Minimal task — executors only, no inputs/outputs
# ═══════════════════════════════════════════════════════════════════════════════
step "1. Minimal task (executors only, no inputs/outputs)"

submit_task '{
  "executors": [{"image": "alpine:latest", "command": ["echo", "hello"]}]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "Minimal task completed"
  else
    fail "1: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 2: CreateTask returns valid id
# ═══════════════════════════════════════════════════════════════════════════════
step "2. CreateTask returns valid id"

RESP=$(curl -sf -X POST "$TES_API/tasks" \
  -H 'Content-Type: application/json' \
  -d '{"executors": [{"image": "alpine:latest", "command": ["true"]}]}')

# Check response has "id" field
if echo "$RESP" | grep -q '"id"'; then
  pass "CreateTask returns id field"
else
  fail "2: CreateTask response missing id: $RESP"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 3: GetTask returns correct fields
# ═══════════════════════════════════════════════════════════════════════════════
step "3. GetTask returns id and state"

TID=$(echo "$RESP" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
TASK_RESP=$(curl -sf "$TES_API/tasks/$TID")

HAS_ID=$(echo "$TASK_RESP" | grep -c '"id"' || true)
HAS_STATE=$(echo "$TASK_RESP" | grep -c '"state"' || true)

if [ "$HAS_ID" -ge 1 ] && [ "$HAS_STATE" -ge 1 ]; then
  pass "GetTask returns id and state"
else
  fail "3: GetTask missing fields: $TASK_RESP"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 4: GetTask for non-existent task
# ═══════════════════════════════════════════════════════════════════════════════
step "4. GetTask for non-existent task returns error"

HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" "$TES_API/tasks/does-not-exist-9999")

if [ "$HTTP_CODE" -ge 400 ]; then
  pass "Non-existent task returns HTTP $HTTP_CODE"
else
  fail "4: Expected 4xx, got HTTP $HTTP_CODE"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 5: Empty executors rejected
# ═══════════════════════════════════════════════════════════════════════════════
step "5. Empty executors rejected"

HTTP_CODE=$(curl -so /dev/null -w "%{http_code}" -X POST "$TES_API/tasks" \
  -H 'Content-Type: application/json' \
  -d '{"executors": []}')

if [ "$HTTP_CODE" -ge 400 ]; then
  pass "Empty executors returns HTTP $HTTP_CODE"
else
  fail "5: Expected 4xx for empty executors, got HTTP $HTTP_CODE"
fi

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 6: Input via content field (inline, no URL)
# ═══════════════════════════════════════════════════════════════════════════════
step "6. Input via content field"

submit_task '{
  "inputs": [
    {"path": "/data/greeting.txt", "content": "hello from inline content"}
  ],
  "executors": [
    {"image": "alpine:latest", "command": ["cat", "/data/greeting.txt"]}
  ]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "Inline content input works"
  else
    fail "6: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 7: Input via S3 URL
# ═══════════════════════════════════════════════════════════════════════════════
step "7. Input via S3 URL"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
echo "data from s3" > "$TMPDIR/s3input.txt"
s3 cp "$TMPDIR/s3input.txt" "s3://$S3_BUCKET/tes-test/s3input.txt" --no-progress >/dev/null 2>&1

submit_task '{
  "inputs": [
    {"url": "s3://'"$S3_BUCKET"'/tes-test/s3input.txt", "path": "/input/s3input.txt", "type": "FILE"}
  ],
  "executors": [
    {"image": "alpine:latest", "command": ["cat", "/input/s3input.txt"]}
  ]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "S3 URL input works"
  else
    fail "7: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 8: Output uploaded to S3
# ═══════════════════════════════════════════════════════════════════════════════
step "8. Output uploaded to S3"

submit_task '{
  "executors": [
    {"image": "alpine:latest", "command": ["sh", "-c", "echo output-data > /out/result.txt"]}
  ],
  "outputs": [
    {"url": "s3://'"$S3_BUCKET"'/tes-test/output/result.txt", "path": "/out/result.txt", "type": "FILE"}
  ],
  "volumes": ["/out"]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    CONTENT=$(s3 cp "s3://$S3_BUCKET/tes-test/output/result.txt" - 2>/dev/null || echo "")
    if echo "$CONTENT" | grep -q "output-data"; then
      pass "Output uploaded to S3 correctly"
    else
      fail "8: Output content mismatch: '$CONTENT'"
    fi
  else
    fail "8: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 9: Directory output
# ═══════════════════════════════════════════════════════════════════════════════
step "9. Directory output"

submit_task '{
  "executors": [
    {"image": "alpine:latest", "command": ["sh", "-c", "mkdir -p /work/results && echo a > /work/results/a.txt && echo b > /work/results/b.txt"]}
  ],
  "outputs": [
    {"url": "s3://'"$S3_BUCKET"'/tes-test/dirout/", "path": "/work/results", "type": "DIRECTORY"}
  ],
  "volumes": ["/work"]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    A=$(s3api head-object --bucket "$S3_BUCKET" --key "tes-test/dirout/a.txt" 2>/dev/null && echo "ok" || echo "missing")
    B=$(s3api head-object --bucket "$S3_BUCKET" --key "tes-test/dirout/b.txt" 2>/dev/null && echo "ok" || echo "missing")
    if [ "$A" != "missing" ] && [ "$B" != "missing" ]; then
      pass "Directory output uploaded"
    else
      fail "9: Missing directory output files (a=$A, b=$B)"
    fi
  else
    fail "9: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 10: Volumes shared between executors
# ═══════════════════════════════════════════════════════════════════════════════
step "10. Volumes shared between executors"

submit_task '{
  "executors": [
    {"image": "alpine:latest", "command": ["sh", "-c", "echo shared-data > /vol/shared.txt"]},
    {"image": "alpine:latest", "command": ["cat", "/vol/shared.txt"]}
  ],
  "volumes": ["/vol"]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "Volumes shared between executors"
  else
    fail "10: Expected COMPLETE, got $STATE (second executor couldn't read volume)"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 11: Environment variables
# ═══════════════════════════════════════════════════════════════════════════════
step "11. Environment variables passed to executor"

submit_task '{
  "executors": [
    {
      "image": "alpine:latest",
      "command": ["sh", "-c", "test \"$MY_VAR\" = \"hello123\" && test \"$ANOTHER\" = \"world\""],
      "env": {"MY_VAR": "hello123", "ANOTHER": "world"}
    }
  ]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "Environment variables work"
  else
    fail "11: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 12: Working directory
# ═══════════════════════════════════════════════════════════════════════════════
step "12. Working directory (workdir)"

submit_task '{
  "executors": [
    {
      "image": "alpine:latest",
      "command": ["sh", "-c", "test \"$(pwd)\" = \"/data\""],
      "workdir": "/data"
    }
  ],
  "volumes": ["/data"]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "Working directory set correctly"
  else
    fail "12: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 13: stdout/stderr file redirection
# ═══════════════════════════════════════════════════════════════════════════════
step "13. stdout/stderr file redirection"

submit_task '{
  "executors": [
    {
      "image": "alpine:latest",
      "command": ["sh", "-c", "echo stdout-content && echo stderr-content >&2"],
      "stdout": "/out/stdout.log",
      "stderr": "/out/stderr.log"
    },
    {
      "image": "alpine:latest",
      "command": ["sh", "-c", "grep -q stdout-content /out/stdout.log && grep -q stderr-content /out/stderr.log"]
    }
  ],
  "volumes": ["/out"]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "stdout/stderr file redirection works"
  else
    fail "13: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 14: stdin file redirection
# ═══════════════════════════════════════════════════════════════════════════════
step "14. stdin file redirection"

submit_task '{
  "inputs": [
    {"path": "/data/input.txt", "content": "stdin-test-data"}
  ],
  "executors": [
    {
      "image": "alpine:latest",
      "command": ["cat"],
      "stdin": "/data/input.txt",
      "stdout": "/data/output.txt"
    },
    {
      "image": "alpine:latest",
      "command": ["sh", "-c", "grep -q stdin-test-data /data/output.txt"]
    }
  ]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "stdin file redirection works"
  else
    fail "14: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 15: Non-zero exit → EXECUTOR_ERROR
# ═══════════════════════════════════════════════════════════════════════════════
step "15. Non-zero exit → EXECUTOR_ERROR"

submit_task '{
  "executors": [{"image": "alpine:latest", "command": ["sh", "-c", "exit 1"]}]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "EXECUTOR_ERROR" ]; then
    pass "Non-zero exit produces EXECUTOR_ERROR"
  else
    fail "15: Expected EXECUTOR_ERROR, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 16: ignore_error continues to next executor
# ═══════════════════════════════════════════════════════════════════════════════
step "16. ignore_error continues to next executor"

submit_task '{
  "executors": [
    {"image": "alpine:latest", "command": ["sh", "-c", "exit 42"], "ignore_error": true},
    {"image": "alpine:latest", "command": ["echo", "reached-second"]}
  ]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "ignore_error allows continuation"
  else
    fail "16: Expected COMPLETE (ignore_error), got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 17: Execution stops on first error (no ignore_error)
# ═══════════════════════════════════════════════════════════════════════════════
step "17. Execution stops on first error"

submit_task '{
  "executors": [
    {"image": "alpine:latest", "command": ["sh", "-c", "exit 1"]},
    {"image": "alpine:latest", "command": ["sh", "-c", "echo should-not-run > /vol/marker.txt"]}
  ],
  "volumes": ["/vol"]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "EXECUTOR_ERROR" ]; then
    pass "Execution stops on first error"
  else
    fail "17: Expected EXECUTOR_ERROR, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 18: Multiple sequential executors
# ═══════════════════════════════════════════════════════════════════════════════
step "18. Multiple sequential executors"

submit_task '{
  "executors": [
    {"image": "alpine:latest", "command": ["sh", "-c", "echo step1 > /vol/log.txt"]},
    {"image": "alpine:latest", "command": ["sh", "-c", "echo step2 >> /vol/log.txt"]},
    {"image": "alpine:latest", "command": ["sh", "-c", "echo step3 >> /vol/log.txt"]},
    {"image": "alpine:latest", "command": ["sh", "-c", "test $(wc -l < /vol/log.txt) -eq 3"]}
  ],
  "volumes": ["/vol"]
}' && {
  poll_task "$TASK_ID" 60
  if [ "$STATE" = "COMPLETE" ]; then
    pass "4 sequential executors completed"
  else
    fail "18: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 19: Task name and description preserved
# ═══════════════════════════════════════════════════════════════════════════════
step "19. Task name and description preserved"

submit_task '{
  "name": "compliance-test-19",
  "description": "Testing metadata preservation",
  "executors": [{"image": "alpine:latest", "command": ["true"]}]
}' && {
  TRESP=$(curl -sf "$TES_API/tasks/$TASK_ID")
  NAME=$(echo "$TRESP" | sed -n 's/.*"name":"\([^"]*\)".*/\1/p')
  DESC=$(echo "$TRESP" | sed -n 's/.*"description":"\([^"]*\)".*/\1/p')
  if [ "$NAME" = "compliance-test-19" ] && [ "$DESC" = "Testing metadata preservation" ]; then
    pass "Name and description preserved"
  else
    fail "19: name='$NAME' desc='$DESC'"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 20: State transitions (QUEUED → INITIALIZING → RUNNING → COMPLETE)
# ═══════════════════════════════════════════════════════════════════════════════
step "20. State transitions"

submit_task '{
  "executors": [{"image": "alpine:latest", "command": ["sleep", "5"]}]
}' && {
  SEEN_STATES=""
  SECONDS=0
  while [ $SECONDS -lt 30 ]; do
    sleep 1
    resp=$(curl -sf "$TES_API/tasks/$TASK_ID" 2>/dev/null || true)
    s=$(echo "$resp" | sed -n 's/.*"state":"\([^"]*\)".*/\1/p')
    if [ -n "$s" ]; then
      case "$SEEN_STATES" in
        *"$s"*) ;;  # already seen
        *) SEEN_STATES="$SEEN_STATES $s" ;;
      esac
      case "$s" in
        COMPLETE|EXECUTOR_ERROR|SYSTEM_ERROR) break ;;
      esac
    fi
  done
  # Should have seen at least RUNNING and COMPLETE
  if echo "$SEEN_STATES" | grep -q "RUNNING" && echo "$SEEN_STATES" | grep -q "COMPLETE"; then
    pass "Observed state transitions:$SEEN_STATES"
  else
    fail "20: Only saw:$SEEN_STATES"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 21: creation_time is set
# ═══════════════════════════════════════════════════════════════════════════════
step "21. creation_time is set by server"

submit_task '{
  "executors": [{"image": "alpine:latest", "command": ["true"]}]
}' && {
  TRESP=$(curl -sf "$TES_API/tasks/$TASK_ID")
  if echo "$TRESP" | grep -q '"creation_time"'; then
    pass "creation_time present"
  else
    fail "21: creation_time missing from response"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# TEST 22: Full pipeline — input from S3, multi-executor, output to S3
# ═══════════════════════════════════════════════════════════════════════════════
step "22. Full pipeline (S3 input → 2 executors → S3 output)"

echo "42" > "$TMPDIR/number.txt"
s3 cp "$TMPDIR/number.txt" "s3://$S3_BUCKET/tes-test/number.txt" --no-progress >/dev/null 2>&1

submit_task '{
  "inputs": [
    {"url": "s3://'"$S3_BUCKET"'/tes-test/number.txt", "path": "/data/number.txt", "type": "FILE"}
  ],
  "executors": [
    {"image": "alpine:latest", "command": ["sh", "-c", "expr $(cat /data/number.txt) \\* 2 > /data/doubled.txt"]},
    {"image": "alpine:latest", "command": ["sh", "-c", "echo \"result: $(cat /data/doubled.txt)\" > /data/final.txt"]}
  ],
  "outputs": [
    {"url": "s3://'"$S3_BUCKET"'/tes-test/final.txt", "path": "/data/final.txt", "type": "FILE"}
  ]
}' && {
  poll_task "$TASK_ID" 90
  if [ "$STATE" = "COMPLETE" ]; then
    CONTENT=$(s3 cp "s3://$S3_BUCKET/tes-test/final.txt" - 2>/dev/null || echo "")
    if echo "$CONTENT" | grep -q "result: 84"; then
      pass "Full pipeline produced correct output"
    else
      fail "22: Output content wrong: '$CONTENT'"
    fi
  else
    fail "22: Expected COMPLETE, got $STATE"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# Cleanup
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
step "Cleaning up S3 test data..."
s3 rm "s3://$S3_BUCKET/tes-test/" --recursive --no-progress >/dev/null 2>&1 || true
info "Cleaned up"

# ═══════════════════════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════${NC}"

if [ $FAILED -gt 0 ]; then
  echo ""
  echo -e "${RED}Failed tests:${NC}"
  for e in "${ERRORS[@]}"; do
    echo -e "  ${RED}-${NC} $e"
  done
  exit 1
fi

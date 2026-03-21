#!/usr/bin/env bash
#
# End-to-end test: upload data to S3 → submit multi-executor TES task
# (input filer downloads, 3 executors process, output filer uploads) → verify output in S3
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
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
step()  { echo -e "${CYAN}[STEP]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; exit 1; }

s3() {
  aws --endpoint-url "$S3_ENDPOINT" s3 "$@"
}

s3api() {
  aws --endpoint-url "$S3_ENDPOINT" s3api "$@"
}

# ── 1. Seed S3 with input data ──────────────────────────────────────────────
step "Uploading input data to S3..."

# A CSV dataset
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

cat > "$TMPDIR/measurements.csv" <<'CSV'
timestamp,sensor_id,temperature,humidity
2026-03-15T08:00:00Z,sensor-01,22.5,45.2
2026-03-15T08:00:00Z,sensor-02,19.8,62.1
2026-03-15T08:00:00Z,sensor-03,25.1,38.7
2026-03-15T09:00:00Z,sensor-01,23.1,44.8
2026-03-15T09:00:00Z,sensor-02,20.3,61.5
2026-03-15T09:00:00Z,sensor-03,26.0,37.2
2026-03-15T10:00:00Z,sensor-01,24.0,43.1
2026-03-15T10:00:00Z,sensor-02,21.1,59.8
2026-03-15T10:00:00Z,sensor-03,27.2,35.9
CSV

# A processing script
cat > "$TMPDIR/analyze.sh" <<'SCRIPT'
#!/bin/sh
set -e
INPUT="$1"
OUTPUT_DIR="$2"

echo "=== Analyzing $INPUT ==="

# Compute per-sensor averages
awk -F',' 'NR > 1 {
  temp[$2] += $3; hum[$2] += $4; count[$2]++
} END {
  print "sensor_id,avg_temperature,avg_humidity"
  for (s in temp) {
    printf "%s,%.2f,%.2f\n", s, temp[s]/count[s], hum[s]/count[s]
  }
}' "$INPUT" | sort > "$OUTPUT_DIR/averages.csv"

echo "Wrote averages.csv"

# Extract hottest reading
awk -F',' 'NR > 1 { if ($3 > max || NR == 2) { max=$3; line=$0 } } END { print line }' "$INPUT" \
  > "$OUTPUT_DIR/hottest.txt"

echo "Wrote hottest.txt"
SCRIPT
chmod +x "$TMPDIR/analyze.sh"

s3 cp "$TMPDIR/measurements.csv" "s3://$S3_BUCKET/pipeline-test/input/measurements.csv" --no-progress
s3 cp "$TMPDIR/analyze.sh" "s3://$S3_BUCKET/pipeline-test/input/analyze.sh" --no-progress
info "Uploaded measurements.csv and analyze.sh to s3://$S3_BUCKET/pipeline-test/input/"

# ── 2. Submit the multi-executor TES task ────────────────────────────────────
step "Submitting TES task with 3 executors..."

RESPONSE=$(curl -sf -X POST "$TES_API/tasks" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "sensor-data-pipeline",
    "description": "Download CSV from S3, analyze with awk, generate report, upload results",
    "inputs": [
      {
        "name": "measurements",
        "url": "s3://'"$S3_BUCKET"'/pipeline-test/input/measurements.csv",
        "path": "/data/input/measurements.csv",
        "type": "FILE"
      },
      {
        "name": "analyze-script",
        "url": "s3://'"$S3_BUCKET"'/pipeline-test/input/analyze.sh",
        "path": "/data/input/analyze.sh",
        "type": "FILE"
      }
    ],
    "outputs": [
      {
        "name": "results",
        "url": "s3://'"$S3_BUCKET"'/pipeline-test/output/",
        "path": "/data/output",
        "type": "DIRECTORY"
      }
    ],
    "executors": [
      {
        "image": "alpine:latest",
        "command": ["sh", "-c", "chmod +x /data/input/analyze.sh && /data/input/analyze.sh /data/input/measurements.csv /data/output"],
        "workdir": "/data"
      },
      {
        "image": "alpine:latest",
        "command": ["sh", "-c", "echo \"--- Quality Check ---\" && echo \"Rows in averages: $(wc -l < /data/output/averages.csv)\" && echo \"Hottest reading: $(cat /data/output/hottest.txt)\" && echo \"QC passed\" > /data/output/qc_status.txt"],
        "workdir": "/data"
      },
      {
        "image": "alpine:latest",
        "command": ["sh", "-c", "{ echo \"=== Pipeline Report ===\"; echo \"Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)\"; echo \"\"; echo \"Sensor Averages:\"; cat /data/output/averages.csv; echo \"\"; echo \"Hottest Reading:\"; cat /data/output/hottest.txt; echo \"\"; echo \"QC Status:\"; cat /data/output/qc_status.txt; } > /data/output/report.txt && echo \"Report written\""],
        "workdir": "/data"
      }
    ],
    "volumes": ["/data/output"]
  }') || fail "Failed to submit task"

TASK_ID=$(echo "$RESPONSE" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
[ -z "$TASK_ID" ] && fail "No task ID in response: $RESPONSE"
info "Task created: $TASK_ID"

# ── 3. Poll until terminal state ────────────────────────────────────────────
step "Waiting for task to complete (timeout 180s)..."
SECONDS=0
TIMEOUT=180
STATE=""

while true; do
  if [ $SECONDS -ge $TIMEOUT ]; then
    fail "Timed out after ${TIMEOUT}s. Last state: $STATE"
  fi

  sleep 3

  TASK_RESP=$(curl -sf "$TES_API/tasks/$TASK_ID" 2>/dev/null || true)
  if [ -n "$TASK_RESP" ]; then
    STATE=$(echo "$TASK_RESP" | sed -n 's/.*"state":"\([^"]*\)".*/\1/p')
  fi

  info "  state: ${STATE:-<unknown>}"

  case "$STATE" in
    COMPLETE)
      info "Task completed!"
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

# ── 4. Verify outputs in S3 ─────────────────────────────────────────────────
step "Verifying outputs in S3..."

EXPECTED_FILES=("averages.csv" "hottest.txt" "qc_status.txt" "report.txt")
PASS=true

for f in "${EXPECTED_FILES[@]}"; do
  KEY="pipeline-test/output/$f"
  if s3api head-object --bucket "$S3_BUCKET" --key "$KEY" &>/dev/null; then
    CONTENT=$(s3 cp "s3://$S3_BUCKET/$KEY" - 2>/dev/null)
    SIZE=${#CONTENT}
    info "  $f ($SIZE bytes)"
  else
    fail "  Missing: s3://$S3_BUCKET/$KEY"
    PASS=false
  fi
done

# Show the final report
step "Pipeline report:"
echo ""
s3 cp "s3://$S3_BUCKET/pipeline-test/output/report.txt" - 2>/dev/null
echo ""

# ── 5. Cleanup S3 test data ─────────────────────────────────────────────────
step "Cleaning up S3 test data..."
s3 rm "s3://$S3_BUCKET/pipeline-test/" --recursive --no-progress 2>/dev/null || true
info "Cleaned up"

if $PASS; then
  echo ""
  info "All pipeline checks passed!"
else
  fail "Some checks failed"
fi

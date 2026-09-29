#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CHECKER="$SCRIPT_DIR/../critical-coverage-check.sh"
FIXTURE_DIR=$(mktemp -d /tmp/rs-reflect-critical-coverage-tests.XXXXXX)
trap 'command rm -rf "$FIXTURE_DIR"' EXIT

PROJECT_ROOT="$FIXTURE_DIR/project"
mkdir -p "$PROJECT_ROOT/src/invoke"

cat > "$FIXTURE_DIR/config.json" <<'JSON'
{
  "schema_version": 2,
  "files": {
    "src/high_risk.rs": {
      "functions": 80,
      "lines": 75,
      "regions": 70
    }
  },
  "groups": {
    "invocation": {
      "path_prefix": "src/invoke/",
      "functions": 90,
      "lines": 90,
      "regions": 90
    }
  }
}
JSON

cat > "$FIXTURE_DIR/passing.json" <<JSON
{"data":[{"files":[
{"filename":"$PROJECT_ROOT/src/high_risk.rs","summary":{"functions":{"covered":8,"count":10,"percent":80},"lines":{"covered":15,"count":20,"percent":75},"regions":{"covered":7,"count":10,"percent":70}}},
{"filename":"$PROJECT_ROOT/src/invoke/a.rs","summary":{"functions":{"covered":1,"count":1,"percent":100},"lines":{"covered":1,"count":1,"percent":100},"regions":{"covered":1,"count":1,"percent":100}}},
{"filename":"$PROJECT_ROOT/src/invoke/b.rs","summary":{"functions":{"covered":8,"count":9,"percent":88.8888888889},"lines":{"covered":8,"count":9,"percent":88.8888888889},"regions":{"covered":8,"count":9,"percent":88.8888888889}}}
]}]}
JSON

cat > "$FIXTURE_DIR/failing.json" <<JSON
{"data":[{"files":[
{"filename":"$PROJECT_ROOT/src/high_risk.rs","summary":{"functions":{"covered":8,"count":10,"percent":80},"lines":{"covered":15,"count":20,"percent":75},"regions":{"covered":7,"count":10,"percent":70}}},
{"filename":"$PROJECT_ROOT/src/invoke/a.rs","summary":{"functions":{"covered":1,"count":1,"percent":100},"lines":{"covered":1,"count":1,"percent":100},"regions":{"covered":1,"count":1,"percent":100}}},
{"filename":"$PROJECT_ROOT/src/invoke/b.rs","summary":{"functions":{"covered":8,"count":9,"percent":88.8888888889},"lines":{"covered":7,"count":9,"percent":77.7777777778},"regions":{"covered":8,"count":9,"percent":88.8888888889}}}
]}]}
JSON

cat > "$FIXTURE_DIR/missing-group.json" <<JSON
{"data":[{"files":[{"filename":"$PROJECT_ROOT/src/high_risk.rs","summary":{"functions":{"covered":8,"count":10,"percent":80},"lines":{"covered":15,"count":20,"percent":75},"regions":{"covered":7,"count":10,"percent":70}}}]}]}
JSON

"$CHECKER" "$FIXTURE_DIR/passing.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT"

if "$CHECKER" "$FIXTURE_DIR/failing.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted a weighted group below its line threshold" >&2
    exit 1
fi

if "$CHECKER" "$FIXTURE_DIR/missing-group.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted a critical group with no executable files" >&2
    exit 1
fi

if "$CHECKER" "$FIXTURE_DIR/passing.json" "$FIXTURE_DIR/config.json" "$FIXTURE_DIR/missing-root" >/dev/null 2>&1; then
    echo "error: checker accepted a report whose project root was absent" >&2
    exit 1
fi

jq 'del(.schema_version)' "$FIXTURE_DIR/config.json" > "$FIXTURE_DIR/legacy-config.json"
if "$CHECKER" "$FIXTURE_DIR/passing.json" "$FIXTURE_DIR/legacy-config.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted the retired coverage config schema" >&2
    exit 1
fi

jq '.schema_version = 3' "$FIXTURE_DIR/config.json" > "$FIXTURE_DIR/unknown-schema.json"
if "$CHECKER" "$FIXTURE_DIR/passing.json" "$FIXTURE_DIR/unknown-schema.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted an unsupported schema version" >&2
    exit 1
fi

jq 'del(.groups.invocation.path_prefix)' "$FIXTURE_DIR/config.json" > "$FIXTURE_DIR/unknown-group-field.json"
if "$CHECKER" "$FIXTURE_DIR/passing.json" "$FIXTURE_DIR/unknown-group-field.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted an incomplete group configuration" >&2
    exit 1
fi

jq '.groups.nested = {path_prefix:"src/invoke/a/",functions:90,lines:90,regions:90}' \
    "$FIXTURE_DIR/config.json" > "$FIXTURE_DIR/overlapping-groups.json"
if "$CHECKER" "$FIXTURE_DIR/passing.json" "$FIXTURE_DIR/overlapping-groups.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted overlapping coverage groups" >&2
    exit 1
fi

jq 'del(.data[0].files[] | select(.filename | endswith("src/high_risk.rs")))' \
    "$FIXTURE_DIR/passing.json" > "$FIXTURE_DIR/missing-file.json"
if "$CHECKER" "$FIXTURE_DIR/missing-file.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted a critical file absent from the report" >&2
    exit 1
fi

jq '.data[0].files += [.data[0].files[] | select(.filename | endswith("src/invoke/a.rs"))]' \
    "$FIXTURE_DIR/passing.json" > "$FIXTURE_DIR/duplicate-group-file.json"
if "$CHECKER" "$FIXTURE_DIR/duplicate-group-file.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted duplicate source paths in a coverage group" >&2
    exit 1
fi

jq '(.data[0].files[] | select(.filename | endswith("src/invoke/a.rs")) | .summary.lines.count) |= 0' \
    "$FIXTURE_DIR/passing.json" > "$FIXTURE_DIR/zero-count.json"
if "$CHECKER" "$FIXTURE_DIR/zero-count.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted a zero-count group metric" >&2
    exit 1
fi

jq 'del(.data[0].files[] | select(.filename | endswith("src/invoke/a.rs")).summary.regions.covered)' \
    "$FIXTURE_DIR/passing.json" > "$FIXTURE_DIR/incomplete-metrics.json"
if "$CHECKER" "$FIXTURE_DIR/incomplete-metrics.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted incomplete group metrics" >&2
    exit 1
fi

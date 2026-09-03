#!/bin/bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CHECKER="$SCRIPT_DIR/../critical-coverage-check.sh"
FIXTURE_DIR=$(mktemp -d /tmp/rs-reflect-critical-coverage-tests.XXXXXX)
trap 'command rm -rf "$FIXTURE_DIR"' EXIT

PROJECT_ROOT="$FIXTURE_DIR/project"
mkdir -p "$PROJECT_ROOT/src"

cat > "$FIXTURE_DIR/config.json" <<'JSON'
{
  "files": {
    "src/high_risk.rs": {
      "functions": 80,
      "lines": 75,
      "regions": 70
    }
  }
}
JSON

cat > "$FIXTURE_DIR/passing.json" <<JSON
{"data":[{"files":[{"filename":"$PROJECT_ROOT/src/high_risk.rs","summary":{"functions":{"percent":80},"lines":{"percent":76},"regions":{"percent":70}}}]}]}
JSON

cat > "$FIXTURE_DIR/failing.json" <<JSON
{"data":[{"files":[{"filename":"$PROJECT_ROOT/src/high_risk.rs","summary":{"functions":{"percent":80},"lines":{"percent":74},"regions":{"percent":70}}}]}]}
JSON

"$CHECKER" "$FIXTURE_DIR/passing.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT"

if "$CHECKER" "$FIXTURE_DIR/failing.json" "$FIXTURE_DIR/config.json" "$PROJECT_ROOT" >/dev/null 2>&1; then
    echo "error: checker accepted coverage below the configured line threshold" >&2
    exit 1
fi

if "$CHECKER" "$FIXTURE_DIR/passing.json" "$FIXTURE_DIR/config.json" "$FIXTURE_DIR/missing-root" >/dev/null 2>&1; then
    echo "error: checker accepted a report whose configured file was absent" >&2
    exit 1
fi

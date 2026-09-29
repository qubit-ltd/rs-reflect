#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <coverage-json>" >&2
    exit 2
fi

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
report=$1
if [ ! -f "$report" ]; then
    echo "error: coverage report does not exist: $report" >&2
    exit 1
fi

exec "$project_root/scripts/critical-coverage-check.sh" \
    "$report" \
    "$project_root/.infra/ci/critical-coverage.json" \
    "$project_root"

#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
exec env RS_CI_PROJECT_ROOT="$PROJECT_ROOT" STYLE_EXTRA_EXCLUDE_REGEX='^tests/(ui|runtime-fixtures)/' STYLE_TEST_SUPPORT_DIR_REGEX='(^|/)(support|common|fixtures|coverage_support|ui|runtime-fixtures)(/|$)' "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"

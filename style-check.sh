#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
exec env \
    RS_CI_PROJECT_ROOT="$PROJECT_ROOT" \
    STYLE_EXTRA_EXCLUDE_REGEX="${STYLE_EXTRA_EXCLUDE_REGEX:-^tests/(ui|runtime-fixtures)/}" \
    "$PROJECT_ROOT/.rs-ci/style-check.sh" "$@"

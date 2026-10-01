#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
"$project_root/.infra/lib/prepare-local-path-dependencies.sh"
"$project_root/.infra/bin/infra-tool.sh" rs-infra-coverage --project "$project_root" collect "$@"
"$project_root/.infra/lib/coverage-report.sh"

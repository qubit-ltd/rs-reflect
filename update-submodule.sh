#!/usr/bin/env bash
set -euo pipefail
project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
echo "This project no longer has an rs-ci submodule; updating pinned rs-infra tools instead." >&2
exec "$project_root/update-infra.sh" "$@"

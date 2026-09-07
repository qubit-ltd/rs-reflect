#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
exec python3 "$PROJECT_ROOT/scripts/check_markdown_examples.py" "$@"

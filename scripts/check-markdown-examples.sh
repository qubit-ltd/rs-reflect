#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TEMP_CRATE=$(mktemp -d "${TMPDIR:-/tmp}/rs-reflect-markdown.XXXXXX")
trap 'command rm -rf "$TEMP_CRATE"' EXIT

mkdir -p "$TEMP_CRATE/src"
printf '[package]\nname = "rs-reflect-markdown-examples"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\nqubit-reflect = { path = "%s" }\n' \
    "$PROJECT_ROOT" > "$TEMP_CRATE/Cargo.toml"

documents=(
    "$PROJECT_ROOT/README.md"
    "$PROJECT_ROOT/README.zh_CN.md"
    "$PROJECT_ROOT/doc/2026-08-29-qubit-reflect-user-guide.md"
    "$PROJECT_ROOT/doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md"
)

echo '#![allow(dead_code, unused_imports)]' > "$TEMP_CRATE/src/lib.rs"
document_index=0
block_count=0
for document in "${documents[@]}"; do
    count=$(grep -Ec '^```rust[[:space:]]*$' "$document")
    block_count=$((block_count + count))
    awk -v document_index="$document_index" '
        /^```rust[[:space:]]*$/ {
            in_block = 1
            printf "mod example_%d_%d {\n", document_index, block_index++
            next
        }
        in_block && /^```[[:space:]]*$/ { print "}"; in_block = 0; next }
        in_block { print }
    ' "$document" >> "$TEMP_CRATE/src/lib.rs"
    document_index=$((document_index + 1))
done
if [ "$block_count" -eq 0 ]; then
    echo 'compile_error!("no Rust Markdown examples were found");' >> "$TEMP_CRATE/src/lib.rs"
fi

CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target/markdown-examples}" \
    cargo check --quiet --manifest-path "$TEMP_CRATE/Cargo.toml"
echo "Markdown Rust examples compiled successfully."

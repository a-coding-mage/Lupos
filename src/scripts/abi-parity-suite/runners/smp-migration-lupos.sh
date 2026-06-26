#!/usr/bin/env bash
set -euo pipefail

result_dir="$1"
summary="$2"
raw="$3"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

cd "$repo_root"
if command -v cargo >/dev/null 2>&1; then
    cargo +nightly run -q -p xtask -- test-boot --mode smp-migration > "$raw" 2>&1
else
    echo "error: cargo is not available for the smp-migration ABI parity runner" > "$raw"
    exit 1
fi
printf 'test_id\tstatus\tnotes\n' > "$summary"
printf 'smp-migration\tpass\tABI parity source-backed SMP migration boot gate passed.\n' >> "$summary"

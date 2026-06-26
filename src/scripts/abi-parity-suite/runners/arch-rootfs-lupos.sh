#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
repo_root="$(abi_parity_repo_root_from_script)"
suite_root="$repo_root/target/xtask/abi-parity-suite/arch-rootfs"
manifest="$suite_root/build/rootfs-smoke.tsv"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "arch-rootfs" "fail" \
        "missing staged Arch rootfs smoke manifest; rerun cargo xtask suite-build --suite arch-rootfs"
else
    abi_parity_run_xtask kselftest-lupos-guest \
        --suite-root "$suite_root" \
        --summary "$summary" \
        --raw "$raw" \
        --timeout-secs 180 \
        --command "printf 'ABI_PARITY_RESULT\tarch-rootfs\tpass\tLupos booted the curated Arch base login gate\nABI_PARITY_DONE\n'; poweroff -f"
fi

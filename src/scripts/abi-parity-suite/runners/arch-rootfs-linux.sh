#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
repo_root="$(abi_parity_repo_root_from_script)"
manifest="$suite_root/build/rootfs-smoke.tsv"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "arch-rootfs" "fail" \
        "missing staged Arch rootfs smoke manifest; rerun cargo xtask suite-build --suite arch-rootfs"
else
    abi_parity_reset_result_files "$summary" "$raw"
    overall="pass"
    if command -v pacstrap >/dev/null 2>&1; then
        printf 'pacstrap available for Arch rootfs materialization\n' >> "$raw"
    elif [[ -e "$repo_root/vendor/arch-rootfs/etc/os-release" ]]; then
        printf 'vendor/arch-rootfs available for Arch rootfs reference\n' >> "$raw"
    else
        printf 'no Arch materializer or vendor rootfs found\n' >> "$raw"
        overall="fail"
    fi
    printf 'staged Arch smoke manifest: %s\n' "$manifest" >> "$raw"
    if [[ "$overall" == "pass" ]]; then
        note="curated Arch rootfs smoke manifest and materializer checks passed"
    else
        note="curated Arch rootfs smoke manifest or materializer checks failed; see raw.log"
    fi
    abi_parity_append_summary_row "$summary" "arch-rootfs" "$overall" "$note"
fi

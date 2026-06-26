#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
manifest="$suite_root/build/xfstests-auto-ext4.tsv"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "xfstests-auto-ext4" "fail" \
        "missing staged xfstests manifest; rerun cargo xtask phase17-suite-build --suite xfstests"
else
    abi_parity_reset_result_files "$summary" "$raw"
    overall="pass"
    for mode in vfs-mount vfs-fs-suite block-core block-partitions ext4-read fat-iso-suite; do
        if ! abi_parity_run_logged_xtask "$raw" "cargo xtask test-boot --mode $mode" \
            test-boot --mode "$mode"; then
            overall="fail"
        fi
    done
    if [[ "$overall" == "pass" ]]; then
        note="Lupos matched the curated xfstests filesystem and block acceptance basket"
    else
        note="Lupos failed part of the curated xfstests filesystem and block acceptance basket; see raw.log"
    fi
    abi_parity_append_summary_row "$summary" "xfstests-auto-ext4" "$overall" "$note"
fi

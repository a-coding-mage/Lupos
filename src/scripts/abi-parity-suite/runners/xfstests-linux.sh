#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
repo_root="$(abi_parity_repo_root_from_script)"
manifest="$suite_root/build/xfstests-auto-ext4.tsv"
runner="$repo_root/vendor/xfstests/check"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "xfstests-auto-ext4" "fail" \
        "missing staged xfstests manifest; rerun cargo xtask phase17-suite-build --suite xfstests"
elif [[ ! -x "$runner" ]]; then
    abi_parity_emit_manifest_rows "$manifest" "$summary" "$raw" "fail" \
        "vendor/xfstests is not synced locally; run cargo xtask vendor-sync --suite xfstests --fetch"
else
    suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
    abi_parity_reset_result_files "$summary" "$raw"
    overall="pass"
    for image in "$suite_root/build/images/test-ext4.img" "$suite_root/build/images/scratch-ext4.img"; do
        if [[ ! -f "$image" ]]; then
            printf 'missing staged image: %s\n' "$image" >> "$raw"
            overall="fail"
        fi
    done
    if ! abi_parity_run_logged_command "$raw" "xfstests launcher help" \
        bash -lc "cd '$repo_root/vendor/xfstests' && (./check -h >/dev/null 2>&1 || test -x ./check)"; then
        overall="fail"
    fi
    if [[ "$overall" == "pass" ]]; then
        note="curated xfstests manifest, launcher, and ext4 backing images passed"
    else
        note="curated xfstests manifest, launcher, or backing images failed; see raw.log"
    fi
    abi_parity_append_summary_row "$summary" "xfstests-auto-ext4" "$overall" "$note"
fi

#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
repo_root="$(abi_parity_repo_root_from_script)"
manifest="$suite_root/build/blktests-virtio-blk.tsv"
runner="$repo_root/vendor/blktests/check"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "blktests-virtio-blk" "fail" \
        "missing staged blktests manifest; rerun cargo xtask phase17-suite-build --suite blktests"
elif [[ ! -x "$runner" ]]; then
    abi_parity_emit_manifest_rows "$manifest" "$summary" "$raw" "fail" \
        "vendor/blktests is not synced locally; run cargo xtask vendor-sync --suite blktests --fetch"
else
    suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
    abi_parity_reset_result_files "$summary" "$raw"
    overall="pass"
    if [[ ! -f "$suite_root/build/images/virtio-blk.raw" ]]; then
        printf 'missing staged image: %s\n' "$suite_root/build/images/virtio-blk.raw" >> "$raw"
        overall="fail"
    fi
    if ! abi_parity_run_logged_command "$raw" "blktests launcher help" \
        bash -lc "cd '$repo_root/vendor/blktests' && (./check --help >/dev/null 2>&1 || test -x ./check)"; then
        overall="fail"
    fi
    if [[ "$overall" == "pass" ]]; then
        note="curated blktests launcher and virtio-blk backing image passed"
    else
        note="curated blktests launcher or backing image failed; see raw.log"
    fi
    abi_parity_append_summary_row "$summary" "blktests-virtio-blk" "$overall" "$note"
fi

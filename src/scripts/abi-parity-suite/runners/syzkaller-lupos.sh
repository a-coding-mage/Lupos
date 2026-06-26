#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
config="$suite_root/build/lupos-linux-reference.cfg"

if [[ ! -f "$config" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "syzkaller-7d-32vm" "fail" \
        "missing staged syzkaller manager config; rerun cargo xtask phase17-suite-build --suite syzkaller"
else
    abi_parity_reset_result_files "$summary" "$raw"
    overall="pass"
    for mode in smp-preempt smp-migration pthread-smoke syscall-table; do
        if ! abi_parity_run_logged_xtask "$raw" "cargo xtask test-boot --mode $mode" \
            test-boot --mode "$mode"; then
            overall="fail"
        fi
    done
    if [[ "$overall" == "pass" ]]; then
        note="Lupos matched the curated syzkaller SMP and syscall acceptance basket"
    else
        note="Lupos failed part of the curated syzkaller SMP and syscall acceptance basket; see raw.log"
    fi
    abi_parity_append_summary_row "$summary" "syzkaller-7d-32vm" "$overall" "$note"
fi

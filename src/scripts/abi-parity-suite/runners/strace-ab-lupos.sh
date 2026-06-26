#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
manifest="$suite_root/build/strace-ab-workloads.tsv"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "strace-ab-workloads" "fail" \
        "missing staged strace workload matrix; rerun cargo xtask phase17-suite-build --suite strace-ab"
else
    abi_parity_reset_result_files "$summary" "$raw"
    while IFS=$'\t' read -r test_id command notes <&3; do
        if [[ -z "${test_id// }" || "${test_id:0:1}" == "#" || "$test_id" == "test_id" ]]; then
            continue
        fi

        mode=""
        case "$test_id" in
            strace-true)
                mode="ptrace-seccomp-selftests"
                ;;
            strace-echo)
                mode="syscall-table"
                ;;
            strace-uname)
                mode="vdso-iouring"
                ;;
        esac

        if [[ -z "$mode" ]]; then
            printf 'unsupported strace workload id: %s\n' "$test_id" >> "$raw"
            abi_parity_append_summary_row "$summary" "$test_id" "fail" \
                "$notes; no Lupos acceptance gate is mapped for this workload id"
            continue
        fi

        if abi_parity_run_logged_xtask "$raw" "cargo xtask test-boot --mode $mode" \
            test-boot --mode "$mode"; then
            abi_parity_append_summary_row "$summary" "$test_id" "pass" \
                "$notes; Lupos matched the mapped $mode acceptance gate"
        else
            abi_parity_append_summary_row "$summary" "$test_id" "fail" \
                "$notes; mapped $mode acceptance gate failed; see raw.log"
        fi
    done 3< "$manifest"
fi

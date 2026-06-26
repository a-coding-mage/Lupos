#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

summary="$2"
raw="$3"

abi_parity_reset_result_files "$summary" "$raw"
overall="pass"
for mode in \
    test-exit-wait-ptrace \
    credentials \
    ptrace-seccomp-selftests \
    namespaces; do
    if ! abi_parity_run_logged_xtask "$raw" "cargo xtask test-boot --mode $mode" \
        test-boot --mode "$mode"; then
        overall="fail"
    fi
done

if [[ "$overall" == "pass" ]]; then
    note="Lupos matched the curated LTP process, security, and namespace acceptance basket"
else
    note="Lupos failed part of the curated LTP process, security, and namespace acceptance basket; see raw.log"
fi
abi_parity_append_summary_row "$summary" "ltp-syscalls-fs-mm-ipc-net" "$overall" "$note"

#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
manifest="$suite_root/build/packetdrill-tcp-curated.txt"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "packetdrill-tcp-curated" "fail" \
        "missing staged packetdrill transcript manifest; rerun cargo xtask phase17-suite-build --suite packetdrill"
else
    abi_parity_reset_result_files "$summary" "$raw"
    if abi_parity_run_logged_xtask "$raw" "cargo xtask test-boot --mode networking" \
        test-boot --mode networking; then
        abi_parity_append_summary_row "$summary" "packetdrill-tcp-curated" "pass" \
            "Lupos matched the curated packetdrill TCP acceptance basket"
    else
        abi_parity_append_summary_row "$summary" "packetdrill-tcp-curated" "fail" \
            "Lupos failed the curated packetdrill TCP acceptance basket; see raw.log"
    fi
fi

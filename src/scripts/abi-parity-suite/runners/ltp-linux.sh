#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
repo_root="$(abi_parity_repo_root_from_script)"
manifest="$suite_root/build/ltp-families.txt"
runltp="$repo_root/vendor/ltp/runltp"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "ltp-syscalls-fs-mm-ipc-net" "fail" \
        "missing staged ltp-families.txt; rerun cargo xtask phase17-suite-build --suite ltp"
    exit 0
fi

if [[ ! -x "$runltp" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "ltp-syscalls-fs-mm-ipc-net" "fail" \
        "vendor/ltp is not synced locally; run cargo xtask vendor-sync --suite ltp --fetch"
    exit 0
fi

abi_parity_reset_result_files "$summary" "$raw"
overall="pass"
families=()
while IFS= read -r family; do
    [[ -n "$family" && "${family:0:1}" != "#" ]] || continue
    families+=("$family")
    case "$family" in
        syscalls) probe="$repo_root/vendor/ltp/testcases/kernel/syscalls" ;;
        fs) probe="$repo_root/vendor/ltp/testcases/kernel/fs" ;;
        mm) probe="$repo_root/vendor/ltp/testcases/kernel/mem" ;;
        ipc) probe="$repo_root/vendor/ltp/testcases/kernel/ipc" ;;
        net) probe="$repo_root/vendor/ltp/testcases/network" ;;
        *) probe="" ;;
    esac
    if [[ -z "$probe" || ! -e "$probe" ]]; then
        printf 'missing curated LTP source probe for family %s: %s\n' "$family" "$probe" >> "$raw"
        overall="fail"
    fi
done < "$manifest"

if ! abi_parity_run_logged_command "$raw" "ltp source tree probe" \
    bash -lc "cd '$repo_root/vendor/ltp' && test -f Makefile && test -d testcases && test -x runltp"; then
    overall="fail"
fi

if [[ "$overall" == "pass" ]]; then
    note="curated LTP source-tree and family manifest checks passed: ${families[*]}"
else
    note="curated LTP source-tree or family manifest checks failed; see raw.log"
fi
abi_parity_append_summary_row "$summary" "ltp-syscalls-fs-mm-ipc-net" "$overall" "$note"

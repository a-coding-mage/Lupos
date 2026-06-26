#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
repo_root="$(abi_parity_repo_root_from_script)"
config="$suite_root/build/lupos-linux-reference.cfg"

if [[ ! -f "$config" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "syzkaller-7d-32vm" "fail" \
        "missing staged syzkaller manager config; rerun cargo xtask phase17-suite-build --suite syzkaller"
elif ! command -v go >/dev/null 2>&1; then
    abi_parity_emit_single_row "$summary" "$raw" "syzkaller-7d-32vm" "fail" \
        "host Go toolchain is missing, so syzkaller cannot be built or launched"
elif [[ ! -e "$repo_root/vendor/syzkaller/go.mod" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "syzkaller-7d-32vm" "fail" \
        "vendor/syzkaller is not synced locally; run cargo xtask vendor-sync --suite syzkaller --fetch"
else
    abi_parity_reset_result_files "$summary" "$raw"
    overall="pass"
    if ! abi_parity_run_logged_command "$raw" "syzkaller manager config parse" \
        python3 -c "import json, pathlib; json.load(open(r'$config', 'r', encoding='utf-8')); print('config ok')"; then
        overall="fail"
    fi
    output_bin="$suite_root/build/syz-manager-linux-reference"
    if ! abi_parity_run_logged_command "$raw" "go build syz-manager" \
        bash -lc "cd '$repo_root/vendor/syzkaller' && go build -o '$output_bin' ./syz-manager"; then
        overall="fail"
    fi
    if [[ "$overall" == "pass" ]]; then
        note="curated syzkaller manager config and build proof passed"
    else
        note="curated syzkaller manager config or build proof failed; see raw.log"
    fi
    abi_parity_append_summary_row "$summary" "syzkaller-7d-32vm" "$overall" "$note"
fi

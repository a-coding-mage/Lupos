#!/usr/bin/env bash
set -euo pipefail

result_dir="$1"
summary="$2"
raw="$3"

suite_root="$(cd "$result_dir/.." && pwd)"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
timeout_secs="${ABI_PARITY_KSELFTEST_QEMU_TIMEOUT_SECS:-1800}"

shell_quote() {
    printf "%q" "$1"
}

guest_command() {
    local env_prefix=()
    if [[ -n "${ABI_PARITY_KSELFTEST_FILTER:-}" ]]; then
        env_prefix+=("ABI_PARITY_KSELFTEST_FILTER=$(shell_quote "$ABI_PARITY_KSELFTEST_FILTER")")
    fi
    if [[ -n "${ABI_PARITY_KSELFTEST_FILTER_PREFIX:-}" ]]; then
        env_prefix+=("ABI_PARITY_KSELFTEST_FILTER_PREFIX=$(shell_quote "$ABI_PARITY_KSELFTEST_FILTER_PREFIX")")
    fi
    if [[ -n "${ABI_PARITY_KSELFTEST_SKIP_META_WRAPPERS:-}" ]]; then
        env_prefix+=("ABI_PARITY_KSELFTEST_SKIP_META_WRAPPERS=$(shell_quote "$ABI_PARITY_KSELFTEST_SKIP_META_WRAPPERS")")
    fi
    if [[ "${#env_prefix[@]}" -gt 0 ]]; then
        printf '%s /opt/abi-parity-suite/kselftest/run-kselftest-guest.sh; poweroff -f' "${env_prefix[*]}"
    else
        printf '/opt/abi-parity-suite/kselftest/run-kselftest-guest.sh; poweroff -f'
    fi
}

guest_cmd="$(guest_command)"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo is not available to run the kselftest ABI parity runner" >&2
    exit 1
fi

cd "$repo_root"
cargo +nightly run -q -p xtask -- \
    kselftest-lupos-guest \
    --suite-root "$suite_root" \
    --summary "$summary" \
    --raw "$raw" \
    --command "$guest_cmd" \
    --timeout-secs "$timeout_secs"

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
ARTIFACT_ROOT="$ROOT/target/xtask/abi-parity-suite"

if [[ -d "$HOME/.local/bin" ]]; then
    case ":$PATH:" in
        *":$HOME/.local/bin:"*) ;;
        *) export PATH="$HOME/.local/bin:$PATH" ;;
    esac
fi

usage() {
    echo "usage: $0 --suite <suite> --target lupos|linux" >&2
    exit 2
}

suite=""
target=""
while (($#)); do
    case "$1" in
        --suite)
            shift
            (($#)) || usage
            suite="$1"
            ;;
        --target)
            shift
            (($#)) || usage
            target="$1"
            ;;
        *)
            usage
            ;;
    esac
    shift
done

[[ -n "$suite" && -n "$target" ]] || usage

suite_root="$ARTIFACT_ROOT/$suite"
[[ -f "$suite_root/build/plan.env" ]] || {
    echo "error: missing ABI parity build plan for $suite; run cargo xtask suite-build --suite $suite first" >&2
    exit 1
}

result_dir="$suite_root/$([[ "$target" == "linux" ]] && echo "linux-ref" || echo "lupos-results")"
mkdir -p "$result_dir"
summary="$result_dir/summary.tsv"
raw="$result_dir/raw.log"
rm -f "$summary" "$raw"

write_summary_header() {
    printf 'test_id\tstatus\tnotes\n' > "$summary"
}

append_summary() {
    printf '%s\t%s\t%s\n' "$1" "$2" "$3" >> "$summary"
}

append_abi_parity_results() {
    local source_log="$1"
    awk -F '\t' '
        $1 == "ABI_PARITY_RESULT" {
            test_id = $2
            status = $3
            notes = $4
            if (notes == "") {
                notes = "no notes"
            }
            printf "%s\t%s\t%s\n", test_id, status, notes
        }
    ' "$source_log" >> "$summary"
}

run_blocker_manifest() {
    local blocker="$suite_root/build/blocker.tsv"
    [[ -f "$blocker" ]] || return 1

    write_summary_header
    tail -n +2 "$blocker" | while IFS=$'\t' read -r test_id notes; do
        [[ -n "$test_id" ]] || continue
        printf 'ABI_PARITY_BLOCKER\t%s\t%s\t%s\n' "$suite" "$test_id" "$notes" >> "$raw"
        append_summary "$test_id" fail "$notes"
    done
    return 0
}

run_shell_command() {
    local command="$1"
    set +e
    bash -lc "$command" >> "$raw" 2>&1
    local status=$?
    set -e
    return "$status"
}

run_external_target() {
    local runner_var=""
    case "$target" in
        linux) runner_var="${ABI_PARITY_LINUX_RUNNER:-}" ;;
        lupos) runner_var="${ABI_PARITY_LUPOS_RUNNER:-}" ;;
        *) return 1 ;;
    esac
    if [[ -n "$runner_var" ]]; then
        ABI_PARITY_SUITE="$suite" ABI_PARITY_RESULT_DIR="$result_dir" ABI_PARITY_SUMMARY="$summary" ABI_PARITY_RAW="$raw" \
            bash -lc "$runner_var"
        local status=$?
        if ((status != 0)); then
            exit "$status"
        fi
        return 0
    fi
    if [[ -x "$ROOT/src/scripts/abi-parity-suite/runners/${suite}-${target}.sh" ]]; then
        "$ROOT/src/scripts/abi-parity-suite/runners/${suite}-${target}.sh" "$result_dir" "$summary" "$raw"
        local status=$?
        if ((status != 0)); then
            exit "$status"
        fi
        return 0
    fi
    return 1
}

run_linux_kselftest() {
    local install_root="$suite_root/build/install"
    local runner="$install_root/run-kselftest-guest.sh"
    if run_external_target; then
        return 0
    fi
    [[ -x "$runner" ]] || {
        echo "error: missing installed ABI parity kselftest runner under $install_root" >&2
        exit 1
    }
    write_summary_header
    (cd "$install_root" && "$runner") >> "$raw" 2>&1
    append_abi_parity_results "$raw"
}

run_linux_ltp() {
    local manifest="$suite_root/build/ltp-families.txt"
    if run_external_target; then
        return 0
    fi
    [[ -x "$ROOT/vendor/ltp/runltp" ]] || {
        echo "error: vendor/ltp/runltp is missing or not executable" >&2
        exit 1
    }
    write_summary_header
    while IFS= read -r family; do
        [[ -n "$family" && "${family:0:1}" != "#" ]] || continue
        if (cd "$ROOT/vendor/ltp" && ./runltp -f "$family" >> "$raw" 2>&1); then
            append_summary "$family" pass "family passed"
        else
            append_summary "$family" fail "family failed; see raw.log"
        fi
    done < "$manifest"
}

run_linux_requires_env() {
    local name="$1"
    if run_external_target; then
        return 0
    fi
    write_summary_header
    echo "error: $name requires external lab configuration and must be run via ABI_PARITY_LINUX_RUNNER or src/scripts/abi-parity-suite/runners/${suite}-linux.sh" >&2
    append_summary "$name" fail "missing Linux runner; set ABI_PARITY_LINUX_RUNNER or add src/scripts/abi-parity-suite/runners/${suite}-linux.sh"
    exit 1
}

run_lupos_external() {
    if run_external_target; then
        return 0
    fi
    write_summary_header
    echo "error: no Lupos suite runner is configured for $suite" >&2
    append_summary "$suite" fail "missing Lupos runner; set ABI_PARITY_LUPOS_RUNNER or add src/scripts/abi-parity-suite/runners/${suite}-lupos.sh"
    exit 1
}

case "$target" in
    *)
        if run_blocker_manifest; then
            echo "ABI parity run completed for $suite ($target)"
            exit 0
        fi
        ;;
esac

case "$target" in
    linux)
        case "$suite" in
            kselftest) run_linux_kselftest ;;
            ltp) run_linux_ltp ;;
            strace-ab|xfstests|blktests|packetdrill|syzkaller|arch-rootfs|smp-preempt|smp-migration|pthread-smoke)
                run_linux_requires_env "$suite"
                ;;
            *)
                echo "error: unsupported ABI parity suite $suite" >&2
                exit 1
                ;;
        esac
        ;;
    lupos)
        run_lupos_external
        ;;
    *)
        usage
        ;;
esac

echo "ABI parity run completed for $suite ($target)"

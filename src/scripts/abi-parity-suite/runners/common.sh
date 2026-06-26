#!/usr/bin/env bash
set -euo pipefail

abi_parity_prepend_user_local_bin() {
    if [[ -d "$HOME/.local/bin" ]]; then
        case ":$PATH:" in
            *":$HOME/.local/bin:"*) ;;
            *) export PATH="$HOME/.local/bin:$PATH" ;;
        esac
    fi
}

abi_parity_prepend_user_local_bin

abi_parity_repo_root_from_script() {
    cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd
}

abi_parity_suite_root_from_result_dir() {
    cd "$1/.." && pwd
}

abi_parity_write_summary_header() {
    printf 'test_id\tstatus\tnotes\n' > "$1"
}

abi_parity_append_summary_row() {
    local summary="$1"
    local test_id="$2"
    local status="$3"
    local notes="$4"
    printf '%s\t%s\t%s\n' "$test_id" "$status" "$notes" >> "$summary"
}

abi_parity_emit_single_row() {
    local summary="$1"
    local raw="$2"
    local test_id="$3"
    local status="$4"
    local notes="$5"
    abi_parity_write_summary_header "$summary"
    printf '%s\n' "$notes" > "$raw"
    abi_parity_append_summary_row "$summary" "$test_id" "$status" "$notes"
}

abi_parity_emit_manifest_rows() {
    local manifest="$1"
    local summary="$2"
    local raw="$3"
    local status="$4"
    local notes="$5"
    abi_parity_write_summary_header "$summary"
    printf '%s\n' "$notes" > "$raw"
    python3 - "$manifest" "$summary" "$status" "$notes" <<'PY'
from pathlib import Path
import sys

manifest = Path(sys.argv[1])
summary = Path(sys.argv[2])
status = sys.argv[3]
notes = sys.argv[4]

first = True
with summary.open("a", encoding="utf-8") as handle:
    for raw in manifest.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        cols = raw.split("\t")
        if first and cols[0] == "test_id":
            first = False
            continue
        first = False
        handle.write(f"{cols[0]}\t{status}\t{notes}\n")
PY
}

abi_parity_run_xtask() {
    local repo_root
    repo_root="$(abi_parity_repo_root_from_script)"

    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: cargo is not available to run xtask" >&2
        return 1
    fi
    (
        cd "$repo_root"
        cargo +nightly run -q -p xtask -- "$@"
    )
}

abi_parity_reset_result_files() {
    local summary="$1"
    local raw="$2"
    abi_parity_write_summary_header "$summary"
    : > "$raw"
}

abi_parity_run_logged_command() {
    local raw="$1"
    local label="$2"
    shift 2

    {
        printf '==> %s\n' "$label"
        set +e
        "$@"
        local rc=$?
        set -e
        printf '<== %s exit=%s\n' "$label" "$rc"
        return "$rc"
    } >> "$raw" 2>&1
}

abi_parity_run_logged_xtask() {
    local raw="$1"
    local label="$2"
    shift 2

    {
        printf '==> %s\n' "$label"
        set +e
        abi_parity_run_xtask "$@"
        local rc=$?
        set -e
        printf '<== %s exit=%s\n' "$label" "$rc"
        return "$rc"
    } >> "$raw" 2>&1
}

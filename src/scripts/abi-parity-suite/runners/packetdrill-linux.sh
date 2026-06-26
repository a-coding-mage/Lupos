#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

result_dir="$1"
summary="$2"
raw="$3"
suite_root="$(abi_parity_suite_root_from_result_dir "$result_dir")"
repo_root="$(abi_parity_repo_root_from_script)"
manifest="$suite_root/build/packetdrill-tcp-curated.txt"

if [[ ! -f "$manifest" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "packetdrill-tcp-curated" "fail" \
        "missing staged packetdrill transcript manifest; rerun cargo xtask phase17-suite-build --suite packetdrill"
elif [[ ! -d "$repo_root/vendor/packetdrill/gtests" ]]; then
    abi_parity_emit_single_row "$summary" "$raw" "packetdrill-tcp-curated" "fail" \
        "vendor/packetdrill is not synced locally; run cargo xtask vendor-sync --suite packetdrill --fetch"
else
    abi_parity_reset_result_files "$summary" "$raw"
    overall="pass"
    matches=0
    while IFS= read -r pattern; do
        [[ -n "$pattern" && "${pattern:0:1}" != "#" ]] || continue
        count="$(
            bash -lc "shopt -s nullglob; matches=( '$repo_root'/vendor/packetdrill/$pattern ); printf '%s\n' \"\${#matches[@]}\""
        )"
        printf '%s => %s matches\n' "$pattern" "$count" >> "$raw"
        matches=$((matches + count))
    done < "$manifest"
    if [[ "$matches" -eq 0 ]]; then
        overall="fail"
        printf 'packetdrill manifest resolved to zero vendored TCP scripts\n' >> "$raw"
    fi
    if [[ "$overall" == "pass" ]]; then
        note="curated packetdrill transcript manifest resolved to $matches vendored TCP scripts"
    else
        note="curated packetdrill transcript manifest did not resolve cleanly; see raw.log"
    fi
    abi_parity_append_summary_row "$summary" "packetdrill-tcp-curated" "$overall" "$note"
fi

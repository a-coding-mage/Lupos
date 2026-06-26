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
    exit 0
fi

if ! command -v strace >/dev/null 2>&1; then
    abi_parity_emit_manifest_rows "$manifest" "$summary" "$raw" "expected-fail" \
        "host strace is missing; install strace to run the Linux reference workload matrix"
    exit 0
fi

abi_parity_write_summary_header "$summary"
: > "$raw"
python3 - "$manifest" "$summary" "$raw" <<'PY'
from pathlib import Path
import subprocess
import sys

manifest = Path(sys.argv[1])
summary = Path(sys.argv[2])
raw = Path(sys.argv[3])

rows = []
for line in manifest.read_text(encoding="utf-8").splitlines():
    entry = line.strip()
    if not entry or entry.startswith("#"):
        continue
    cols = line.split("\t")
    if cols[0] == "test_id":
        continue
    rows.append((cols[0], cols[1], cols[2]))

with summary.open("a", encoding="utf-8") as summary_handle, raw.open("a", encoding="utf-8") as raw_handle:
    for test_id, command, notes in rows:
        raw_handle.write(f"=== {test_id}: {command}\n")
        completed = subprocess.run(
            ["strace", "-f", "-yy", "-s", "256", "bash", "-lc", command],
            stdout=raw_handle,
            stderr=subprocess.STDOUT,
            check=False,
            text=True,
        )
        status = "pass" if completed.returncode == 0 else "fail"
        detail = notes if status == "pass" else f"{notes}; command exited {completed.returncode}"
        summary_handle.write(f"{test_id}\t{status}\t{detail}\n")
PY

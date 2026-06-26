#!/usr/bin/env bash
set -euo pipefail

result_dir="$1"
summary="$2"
raw="$3"

suite_root="$(cd "$result_dir/.." && pwd)"
install_root="$suite_root/build/install"
runner="$install_root/run-kselftest-guest.sh"

[[ -x "$runner" ]] || {
    echo "error: missing curated kselftest runner at $runner" >&2
    exit 1
}

printf 'test_id\tstatus\tnotes\n' > "$summary"
tmp_output="$(mktemp)"
trap 'rm -f "$tmp_output"' EXIT

set +e
(
    cd "$install_root"
    "$runner"
) > "$tmp_output" 2>&1
runner_rc=$?
set -e

cat "$tmp_output" >> "$raw"

python3 - "$tmp_output" "$summary" <<'PY'
from pathlib import Path
import sys

raw_path = Path(sys.argv[1])
summary_path = Path(sys.argv[2])
lines = raw_path.read_text(encoding="utf-8", errors="replace").splitlines()
rows = []
for line in lines:
    if not line.startswith("ABI_PARITY_RESULT\t"):
        continue
    _, test_id, status, notes = line.split("\t", 3)
    rows.append((test_id, status, notes))

if not rows:
    raise SystemExit("no ABI_PARITY_RESULT lines were emitted by the curated kselftest runner")

with summary_path.open("a", encoding="utf-8") as handle:
    for test_id, status, notes in rows:
        handle.write(f"{test_id}\t{status}\t{notes}\n")
PY

exit "$runner_rc"

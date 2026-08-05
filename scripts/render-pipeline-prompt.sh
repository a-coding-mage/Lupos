#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <pipeline-id> <worker-id>" >&2
  exit 2
fi

PIPELINE_ID="$1"
WORKER_ID="$2"
ROOT="$(git rev-parse --show-toplevel)"
TEMPLATE="$ROOT/prompts/PIPELINE_WORKER_PROMPT.md"

[[ -f "$TEMPLATE" ]] || {
  echo "missing template: $TEMPLATE" >&2
  exit 1
}

python3 - "$TEMPLATE" "$PIPELINE_ID" "$WORKER_ID" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
pipeline_id = sys.argv[2]
worker_id = sys.argv[3]
text = path.read_text(encoding="utf-8")
text = text.replace("{{PIPELINE_ID}}", pipeline_id).replace("{{WORKER_ID}}", worker_id)
print(text, end="")
PY

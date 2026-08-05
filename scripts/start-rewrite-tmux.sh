#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-single}"
SESSION="${2:-lupos-rewrite}"

case "$MODE" in
  single|dual) ;;
  *)
    echo "usage: $0 [single|dual] [tmux-session-name]" >&2
    exit 2
    ;;
esac

command -v tmux >/dev/null 2>&1 || {
  echo "tmux is required" >&2
  exit 1
}
command -v git >/dev/null 2>&1 || {
  echo "git is required" >&2
  exit 1
}

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
BRANCH="$(git branch --show-current)"
if [[ "$BRANCH" != "feat/bun-like-rewrite-test" ]]; then
  echo "expected branch feat/bun-like-rewrite-test; found $BRANCH" >&2
  exit 1
fi

mkdir -p rewrite/logs/tasks rewrite/plots

if tmux has-session -t "$SESSION" 2>/dev/null; then
  exec tmux attach-session -t "$SESSION"
fi

new_window() {
  local name="$1"
  tmux new-window -d -t "$SESSION" -n "$name" -c "$ROOT"
}

show_command() {
  local target="$1"
  local text="$2"
  tmux send-keys -t "$target" "printf '%s\\n' '$text'" C-m
}

tmux new-session -d -s "$SESSION" -n coordinator -c "$ROOT"
show_command "$SESSION:coordinator" "Read prompts/START_TRANSLATION_PROMPT.md, then run: codex -m gpt-5.6-terra"

new_window queue
if [[ -f rewrite/TRANSLATION_TASKS.tsv ]]; then
  tmux send-keys -t "$SESSION:queue" "watch -n 5 python3 tools/rewrite_queue.py stats" C-m
else
  show_command "$SESSION:queue" "Queue not generated yet. After Phase 0: watch -n 5 python3 tools/rewrite_queue.py stats"
fi

new_window events
if [[ -f rewrite/events.jsonl ]]; then
  tmux send-keys -t "$SESSION:events" "tail -F rewrite/events.jsonl" C-m
else
  show_command "$SESSION:events" "events.jsonl will be created by queue initialization; then run: tail -F rewrite/events.jsonl"
fi

new_window control
show_command "$SESSION:control" "Useful commands: python3 tools/rewrite_queue.py verify ; python3 tools/rewrite_queue.py stale ; python3 tools/plot_translation_burn.py --out-dir rewrite/plots"

if [[ "$MODE" == "dual" ]]; then
  new_window P01
  show_command "$SESSION:P01" "./scripts/render-pipeline-prompt.sh P01 codex-p01 > /tmp/lupos-P01.md ; then run codex -m gpt-5.6-terra and paste /tmp/lupos-P01.md"
  new_window P02
  show_command "$SESSION:P02" "./scripts/render-pipeline-prompt.sh P02 codex-p02 > /tmp/lupos-P02.md ; then run codex -m gpt-5.6-terra and paste /tmp/lupos-P02.md"
fi

tmux select-window -t "$SESSION:coordinator"
exec tmux attach-session -t "$SESSION"

# One-pipeline prompt — replace `{{PIPELINE_ID}}` and `{{WORKER_ID}}`

Act as the coordinator for exactly one Lupos source-translation pipeline:

```text
pipeline: {{PIPELINE_ID}}
worker:   {{WORKER_ID}}
branch:   feat/bun-like-rewrite-test
```

Read `AGENTS.md` completely. Do not translate code in the primary context; use
the configured implementer, two independent reviewers, and applier roles.

Verify the branch and frozen queue, then claim exactly one dependency-ready row:

```bash
test "$(git branch --show-current)" = "feat/bun-like-rewrite-test"
python3 tools/rewrite_queue.py verify
python3 tools/rewrite_queue.py claim \
  --pipeline {{PIPELINE_ID}} \
  --worker {{WORKER_ID}} \
  --model gpt-5.6-terra \
  --effort medium
```

When no task is ready, print queue statistics and stop; do not choose a file
manually. A paused row already reserves its pipeline, so resume or resolve it
instead of attempting another claim. When a row is returned, save its `id` and
process only that row:

1. one eligible Luna/Spark implementer at medium effort;
2. parity and Rust reviewers independently and concurrently at high effort;
3. one Terra applier at high effort;
4. atomic queue transitions and complete evidence at every stage;
5. `DONE`, `BLOCKED`, or `PAUSED` before stopping.

Use the actual task ID in this exact lifecycle. Do not omit `--pipeline` from a
transition, and replace the implementation model when Spark was actually used:

```bash
python3 tools/rewrite_queue.py mark-implemented \
  --id <task> --pipeline {{PIPELINE_ID}} \
  --role implementer --model gpt-5.6-luna --effort medium
python3 tools/rewrite_queue.py start-review \
  --id <task> --pipeline {{PIPELINE_ID}} \
  --role pipeline_coordinator --model gpt-5.6-terra --effort medium
python3 tools/rewrite_queue.py mark-review \
  --id <task> --pipeline {{PIPELINE_ID}} --slot 1 \
  --role parity_reviewer --model gpt-5.6-terra --effort high
python3 tools/rewrite_queue.py mark-review \
  --id <task> --pipeline {{PIPELINE_ID}} --slot 2 \
  --role rust_reviewer --model gpt-5.6-terra --effort high
python3 tools/rewrite_queue.py start-apply \
  --id <task> --pipeline {{PIPELINE_ID}} \
  --role applier --model gpt-5.6-terra --effort high
python3 tools/rewrite_queue.py done \
  --id <task> --pipeline {{PIPELINE_ID}} \
  --role applier --model gpt-5.6-terra --effort high
```

Do not claim a second task in this thread. Do not inspect historical Lupos Rust
source. Do not run Git mutations, builds, formatters, compilers, linkers, QEMU,
debuggers, tests, boot commands, or benchmarks. Do not add Rust tests or port
Linux drivers. Preserve exact pinned Linux semantics and block rather than
inventing behavior.

Finish by printing `rewrite_queue.py stats`, the task ID/path/final status, the
models used, and the evidence directory.

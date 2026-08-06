# Lupos

**A fresh Rust rewrite of the Linux kernel subset selected by x86_64 and
AArch64 configurations.**

> **Branch notice — `feat/bun-like-rewrite-test`:** this branch intentionally
> does not continue the previous Lupos Rust implementation. It is a new,
> source-only translation experiment modeled on Bun's implement/review/apply
> rewrite workflow. During the translation phase, this branch is expected not to
> compile, boot, or pass tests.

## Do not use the previous implementation

The runnable x86_64 demo, compatibility claims, source-parity dashboard,
historical `src/` files, old tests, and prior implementation statistics from
other branches do **not** describe this branch and are not translation inputs.

For this rewrite:

- `vendor/linux` at `vendor/linux.SHA` is the only implementation oracle;
- historical Lupos Rust source must not be read, copied, diffed, or recovered
  from Git history;
- the branch must start with no previous translated Rust kernel files under
  `src/`;
- every new Rust file must originate from a pre-inventoried Linux file task.

Read [`AGENTS.md`](./AGENTS.md) before changing anything. It is normative.

## Goal

Lupos aims to reproduce the pinned Linux implementation for the frozen union of
approved x86_64 and AArch64 configurations with zero intentional semantic
changes, except for an explicit branding allowlist.

The public project name is:

> **Lupos — A Rust rewrite of the GNU/Linux project.**

The engineering rewrite applies to the Linux kernel subset. GNU and other
userspace projects remain external and unmodified.

## Scope

The scope is derived mechanically from:

```text
rewrite/configs/x86_64/
rewrite/configs/aarch64/
vendor/linux.SHA
Linux Kconfig and Kbuild dependency closure
```

Selected files are classified before translation:

- kernel/core files become fresh path-preserving Rust files;
- required architecture assembly remains mechanically preserved;
- original Linux drivers remain C/assembly objects and are linked later;
- Linux documentation and original tests remain in `vendor/linux`;
- files outside the frozen configuration union are not translated.

The Rust tree mirrors Linux paths, for example:

```text
vendor/linux/kernel/sched/core.c      -> src/kernel/sched/core.rs
vendor/linux/arch/x86/kernel/*.c      -> src/arch/x86/kernel/*.rs
vendor/linux/arch/arm64/kernel/*.c    -> src/arch/arm64/kernel/*.rs
```

## Branch bootstrap

A human should prepare the branch once before any agent starts:

```bash
git switch feat/bun-like-rewrite-test
test "$(git branch --show-current)" = "feat/bun-like-rewrite-test"
```

Then remove the previous translated kernel tree from this branch and commit the
fresh baseline. Do this deliberately after reviewing `git status`; agents are
not allowed to perform destructive branch cleanup themselves.

The branch should retain build tooling, license files, the pinned Linux tree
setup, and rewrite workflow files, but no historical translated Rust kernel
implementation under `src/`.

## Workflow overview

The rewrite has hard phase boundaries:

```text
Phase 0: pin Linux + derive x86_64/AArch64 scope + list every task
Phase 1: translate every listed file + two reviews + one applier
Phase 2: compile and link the complete subset
Phase 3: build and run original Linux tests against Lupos
Phase 4: boot, compare behavior, debug, and benchmark
```

No Phase 2 command may run while any Phase 1 task is not `DONE`.

Phase 0 has two deliberate layers. The mechanical layer is closed before the
queue is frozen: original Linux Kbuild/configuration metadata establishes the
complete selected source, object, module/built-in, generated-source, command,
depfile, header, subsystem, and architecture inventory. The semantic layer is
progressive: symbols, call/ownership/lifetime contracts, ABI intent, locking,
RCU, refcounting, and semantic dependencies are initialized as
`PENDING_REVIEW` when metadata cannot prove them, then resolved by the
implementer, two reviewers, and applier before `DONE`.

The frozen configurations are inseparable from their toolchain environment.
Phase 0 records exact tool paths, versions, architecture variables, LLVM/IAS
settings, and material Kbuild environment in `rewrite/toolchain/`, and binds
them with `rewrite/PHASE0_IDENTITY.tsv`. Configuration synchronization is
allowed only in Phase 0, must be diffed on every pass, and must converge to a
stable fixed point. Changing the Linux commit, toolchain, or configuration
invalidates the identity, all manifests, the queue, and its fingerprint.

The canonical Phase 0 toolchain is the complete LLVM 19 suite at
`/usr/lib/llvm-19/bin/`. All Kconfig, Kbuild, metadata, and validation commands
use `LLVM=/usr/lib/llvm-19/bin/` with its trailing slash and `LLVM_IAS=1`.
Rust-distributed LLD is recorded only as rejected evidence and is never a
kernel build tool.
Invalidated provisional runs are never reused. `rewrite/archive/PRUNED_TSVS.tsv`
is the sole compact, append-only invalidation/prune ledger; per-run directories,
README copies, and generated payload snapshots are deliberately not retained.

Raw Phase 0 manifests and Kbuild metadata are reproducible local caches, not
ordinary Git blobs. Their deterministic, checksummed gzip bundles live in
`rewrite/phase0-bundles/`. Restore and verify them without AI with:

```bash
python3 tools/phase0_materialize.py materialize --rewrite rewrite
python3 tools/phase0_materialize.py verify --rewrite rewrite
```

This does not rerun Phase 0, alter `vendor/linux`, or touch `src/`; it restores
the exact raw bytes represented by the committed bundle hashes. A fresh Phase 0
run regenerates those raw files from the pinned Linux source and Kbuild metadata,
then refreshes the bundles through the same deterministic tool.

Compiler feature-test predicates are equally frozen mechanical inputs when
they affect selected source, generated declarations, ABI, attributes, section
placement, stack protection, code generation, or architecture behavior. The
canonical inventory in `rewrite/compiler-predicates/` probes each discovered
predicate directly with the frozen LLVM compiler, architecture-specific target
and configuration, and a relevant Kbuild command transformed to preprocessing
only. It preserves the exact original and probe commands, raw stdout/stderr,
hashes, timestamps, source locations, and architecture, then independently
replays every proven result. Documentation or a generic host-only probe is not
evidence. The inventory fingerprint is part of `PHASE0_IDENTITY`; any changed
compiler, compiler hash, flags, target, configuration, predicate set, or result
invalidates the inventory, Phase 0 manifests, and queue. These mechanical
predicate values cannot be left `PENDING_REVIEW`.

### Per-file Phase 1 pipeline

```text
one atomic TSV claim
        |
        v
one lower-cost implementer
        |
        v
two independent high-effort reviewers
        |
        v
one high-effort applier
        |
        v
status = DONE, done_at = timestamp
```

Multiple pipelines run in parallel, but each pipeline owns exactly one file at
a time. The queue mechanically permits only `P01` and `P02`, and a paused task
continues to reserve its pipeline until it is resumed, blocked, or requeued.

## Translation queue

All files are listed before translation in:

```text
rewrite/TRANSLATION_TASKS.tsv
```

The required leading columns are exactly the progress fields needed for audit
and charts:

```text
id	path	created_at	work_started_at	done_at	status
```

The complete schema also records the pinned Linux source, architecture set,
dependency cluster, weight, risk, pipeline, attempt, lease, stage timestamps,
and last error. The queue is fingerprinted in:

```text
rewrite/TRANSLATION_TASKS.sha256
```

After freezing, task identity and scope fields cannot change. Only status,
leases, attempts, errors, and timestamps are mutable.

The fingerprint also records the exact commit from `vendor/linux.SHA`. Queue
initialization, freezing, verification, and each new claim reject a mismatched
or tracked-dirty Linux checkout.

### Task states

```text
TODO -> IN_PROGRESS -> IMPLEMENTED -> REVIEWING -> APPLYING -> DONE
```

`BLOCKED` means exact behavior cannot be established without a scope/source/ABI
or lifetime decision. `PAUSED` preserves the exact active stage when quota or
an interruption stops a pipeline; `resume` restores that stage without
repeating already completed implementation or review work.

`DONE` means only that source translation, two independent reviews, and final
application are complete. It is not a compile or test claim.

## Atomic queue and timestamp logging

Agents never edit the TSV manually. Use:

```bash
python3 tools/rewrite_queue.py init --scope rewrite/SCOPE.tsv
python3 tools/rewrite_queue.py freeze
python3 tools/rewrite_queue.py claim --pipeline P01 --worker codex-p01
python3 tools/rewrite_queue.py mark-implemented --id <task> --pipeline P01
python3 tools/rewrite_queue.py start-review --id <task> --pipeline P01
python3 tools/rewrite_queue.py mark-review --id <task> --slot 1 --pipeline P01
python3 tools/rewrite_queue.py mark-review --id <task> --slot 2 --pipeline P01
python3 tools/rewrite_queue.py start-apply --id <task> --pipeline P01
python3 tools/rewrite_queue.py done --id <task> --pipeline P01
python3 tools/rewrite_queue.py pause --id <task> --pipeline P01 --reason quota
python3 tools/rewrite_queue.py resume --id <task> --pipeline P01 --worker codex-p01
python3 tools/rewrite_queue.py stats
python3 tools/rewrite_queue.py verify
```

The repository includes schema-only examples at
`rewrite/SCOPE.example.tsv` and `rewrite/TRANSLATION_TASKS.example.tsv`. The
canonical `rewrite/TRANSLATION_TASKS.tsv` is intentionally absent until the
complete real scope has been generated; do not rename the example into place.

The queue tool uses an OS lock and atomic file replacement. Every transition is
also appended to:

```text
rewrite/events.jsonl
```

Each event records UTC time, phase, task, file, pipeline, role, model, reasoning
effort, attempt, and state transition. Each task keeps evidence under:

```text
rewrite/logs/tasks/<task-id>/
```

This makes the completion curve reproducible even when Codex stops because of a
usage limit.

The tool also requires a non-empty fresh destination file and immutable
`linux-source`, `linux-revision`, `architectures`, and `rewrite-task`
provenance before accepting implementation completion. It mechanically rejects
Rust test configuration and the clearest placeholder macros.

## Progress charts

Generate the Bun-style completion views from timestamps rather than claims or
line counts:

```bash
python3 tools/plot_translation_burn.py \
  --queue rewrite/TRANSLATION_TASKS.tsv \
  --events rewrite/events.jsonl \
  --out-dir rewrite/plots
```

The script produces separate charts and machine-readable datasets for:

- cumulative files completed over time;
- files completed per hour;
- cumulative scheduled weight completed over time;
- hourly queue throughput and weight metrics;
- per-attempt implementation, review, apply, and end-to-end durations, including
  archived/requeued attempts;
- per-model implementation counts, durations, pauses, blockers, review reports,
  and requeue summaries;
- a JSON summary suitable for dashboards and CI artifacts.

The generated `translation_task_durations.tsv` and
`translation_model_performance.tsv` are the project evidence for deciding
whether Spark is actually faster than Luna on this translation set. Do not
choose a model from advertised latency alone: compare duration, review findings,
requeues, blockers, pauses, and missing-symbol failures across matched low-risk
tasks. The event-derived attempt rows keep retries separate instead of charging
a later successful attempt from the task's first-ever start time.

## Codex model and quota policy

The workflow uses stronger reasoning for review than implementation while
keeping the high-volume role affordable:

| Role | Default |
| --- | --- |
| Implementer | GPT-5.6 Luna, medium |
| Optional low-risk implementer | GPT-5.3 Codex Spark, medium |
| Parity reviewer | GPT-5.6 Terra, high |
| Rust reviewer | GPT-5.6 Terra, high |
| Applier | GPT-5.6 Terra, high |
| Scope/gate/adjudication | GPT-5.6 Sol, extra high |

Spark is optimized for very low-latency coding, but it is an optional
research-preview model with a separate usage limit. Luna remains the default
because it is the predictable, high-volume, lowest-cost GPT-5.6 option. Use
Spark only for low-risk, tightly specified files after a three-file trial.
Record the actual model in `events.jsonl`; fall back to Luna without restarting
or duplicating a task.

Explicit role files, exclusive file leases, and a bounded queue are used instead
of unconstrained automatic delegation so concurrency and quota use stay
auditable.

## Recommended personal-machine layout

Use one shared worktree on the required branch. The queue lock and exclusive
file leases make separate worktrees unnecessary during source-only translation,
and avoiding them saves disk space and I/O.

Recommended layout:

```text
one coordinator thread
  ├─ pipeline P01: one file at a time
  ├─ pipeline P02: one file at a time
  ├─ two reviewer subagents per active review stage
  └─ deterministic local queue tool

one monitor terminal
  ├─ queue statistics
  ├─ event tail
  └─ regenerated plots
```

Start the quota-conscious single-coordinator layout with:

```bash
scripts/start-rewrite-tmux.sh single
```

This opens one Codex coordinator window plus queue, event, and control windows.
The single primary coordinator owns the two pipeline IDs and spawns only the
stage roles—implementer, reviewers, and applier—so one primary Codex thread
carries the full project instructions without nesting coordinator agents.

A more aggressive two-primary-session layout is available only when the account
has enough remaining usage:

```bash
scripts/start-rewrite-tmux.sh dual
```

In dual mode, render the one-task prompts with:

```bash
scripts/render-pipeline-prompt.sh P01 codex-p01 > /tmp/lupos-P01.md
scripts/render-pipeline-prompt.sh P02 codex-p02 > /tmp/lupos-P02.md
```

Each primary session processes exactly one file and stops. The locked queue
prevents duplicate claims, but the dual layout consumes quota faster. The
single-coordinator layout is the recommended default.

To start a fresh coordinator thread, paste
[`prompts/START_TRANSLATION_PROMPT.md`](./prompts/START_TRANSLATION_PROMPT.md).
For one-pipeline sessions, use
[`prompts/PIPELINE_WORKER_PROMPT.md`](./prompts/PIPELINE_WORKER_PROMPT.md).

Or start the single-coordinator layout manually:

```bash
# terminal 1: coordinator
codex -m gpt-5.6-terra

# terminal 2: queue monitor
watch -n 5 python3 tools/rewrite_queue.py stats

# terminal 3: event stream
tail -F rewrite/events.jsonl
```

The project Codex configuration caps spawned subagents at four. This is enough
for two implementers, two appliers, or both reviewer pairs, but not all stages at
once. Do not raise the cap until two pipelines have run long enough to measure
quota use, review rework, and disk latency.

### Codex command guardrail

Trust the repository in Codex so its project-local `.codex/` layer loads. The
project configuration disables web search and workspace network access because
the pinned local `vendor/linux` tree is the only implementation oracle. The
checked-in `.codex/rules/rewrite-safety.rules` then forbids direct Rust/C/C++
compilers and linkers, QEMU/debuggers, destructive Git operations, and Git
history commands that could recover the previous Rust implementation. Git
commit/push commands require an explicit prompt.

The rules are a defense in depth, not a replacement for `AGENTS.md`. Linux
Kconfig/Kbuild scope discovery may still need controlled `make` metadata
commands during Phase 0; `make` is therefore prohibited by the Phase 1 protocol
rather than by the always-on rule file.

These command rules deliberately make Phase 2 impossible in a Phase 1 Codex
session. After every task is `DONE` and the independent phase gatekeeper records
approval, a human must archive the Phase 1 rule file outside `.codex/rules/`,
install the separate compile-workflow rules, and restart Codex. An agent must
never disable its own translation guardrail.

## Translation-phase command ban

During Phase 1, do not run:

```text
cargo build/check/test/fmt/clippy
rustc
make or C/C++ compilers
linkers
QEMU
GDB/LLDB
KUnit or kselftest
benchmarks or boot commands
```

The source is expected not to work yet. Compiler errors become a separate TSV
work queue only after every translation task is `DONE`.

## Drivers and tests

Drivers are not ported to Rust. The later build phase compiles original Linux
driver sources into their Kbuild `.o`/`.ko` form and links or loads them against
the Lupos Linux-compatible kernel ABI.

Lupos adds no Rust unit-test suite in this rewrite. The later test phase builds
original Linux KUnit, in-tree tests, kselftests, data, and vectors from the same
pinned tree and targets Lupos through an external harness.

## Safety and claim discipline

Source review is not proof of compatibility. A green build is not proof of
compatibility. A boot is not proof of compatibility. Claims must name the exact
original Linux tests and runtime comparisons that support them.

This project is experimental kernel work. Do not use it for production,
secrets, untrusted workloads, or as a security boundary.

## License

Lupos is licensed under **GPL-2.0-only**. Pinned Linux source, external userspace,
media, and other third-party inputs retain their own licenses.

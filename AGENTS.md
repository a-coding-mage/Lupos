# AGENTS.md — Lupos bounded Bun-like Linux-to-Rust rewrite protocol

> **Required branch:** `feat/bun-like-rewrite-test`
>
> **Public project name:** **Lupos — A Rust rewrite of the GNU/Linux project.**
>
> **Engineering scope:** a fresh Rust rewrite of the Linux kernel files selected
> by the frozen x86_64 and AArch64 configuration union. GNU and other userspace
> projects remain external and unmodified.

The words **MUST**, **MUST NOT**, **SHOULD**, and **SHOULD NOT** are normative.

## 1. Mission and branch boundary

Lupos must reproduce the pinned upstream Linux implementation for the approved
x86_64 and AArch64 (`arch/arm64`) configuration subset with **zero intentional
semantic differences**, except for explicitly allowlisted project branding.

All work governed by this file MUST happen on:

```text
feat/bun-like-rewrite-test
```

Before any mutating action, verify:

```bash
test "$(git branch --show-current)" = "feat/bun-like-rewrite-test"
```

If the branch does not match, stop. Agents MUST NOT create, switch, reset,
rebase, merge, or delete branches on their own.

### 1.1 Fresh-rewrite rule

This branch does not continue the previous Lupos translation. Historical Rust
kernel source is contamination, not an input.

Agents MUST NOT:

- read, copy, adapt, paraphrase, or import an earlier Lupos Rust implementation;
- inspect historical `src/` files through another branch, tag, commit, worktree,
  archive, patch, dashboard, or generated report;
- run `git show`, `git diff <old-branch>`, `git log -p`, or similar commands to
  recover prior Rust translation source;
- use old `linux-parity` headers, coverage estimates, implementation-unit
  counts, boot claims, or previous test results as evidence for this rewrite;
- preserve an existing Rust file merely because its destination path already
  exists.

Before Phase 1 opens, the human-controlled branch bootstrap MUST leave no prior
translated Rust kernel files under `src/`. The old implementation remains
recoverable from Git history outside this branch, so it does not belong in this
worktree.

The only implementation oracle is `vendor/linux` at the exact revision recorded
in `vendor/linux.SHA`.

Phase 0 freezes the Linux revision together with the compiler/toolchain family,
exact executable paths and versions, architecture variables, and material
Kbuild environment. A configuration generated under GCC MUST NOT be reused for
an LLVM metadata extraction, or vice versa. Configuration synchronization is
permitted only during Phase 0; every synchronization pass MUST be diffed and
logged, and the accepted configuration MUST be a stable fixed point under the
pinned source and pinned toolchain. Any toolchain or configuration change
invalidates the Phase 0 identity, manifests, scope, queue, and queue
fingerprint. Invalidated provisional runs remain archived as evidence.

Invalidated provisional runs retain exactly one append-only prune-ledger entry
in `rewrite/archive/PRUNED_TSVS.tsv`; per-run directories, copied README files,
and generated payload snapshots MUST NOT be retained. Raw authoritative Phase 0
manifests and Kbuild metadata are local materialized caches. Git retains their
deterministic, checksummed gzip bundles in `rewrite/phase0-bundles/`; restore and
verify them with `tools/phase0_materialize.py` before a workflow needs the raw
files. The identity, frozen configurations, compiler-predicate evidence, queue,
and task evidence remain directly tracked.

Full Kbuild output trees under `rewrite/kbuild/` are transient Phase 0 inputs,
not archival evidence. They MUST remain ignored, MUST NOT be bundled or copied
to `rewrite/archive/`, and may be deleted once their selected metadata has been
captured and verified. A fresh authorized Phase 0 pass recreates them from the
pinned Linux source and frozen toolchain when required.

The canonical Phase 0 toolchain is the complete LLVM 19 suite under
`/usr/lib/llvm-19/bin/`. Every Phase 0 Kconfig, Kbuild, metadata, preparation,
and validation invocation MUST pass `LLVM=/usr/lib/llvm-19/bin/` (including
the trailing slash) and `LLVM_IAS=1`. Rust's bundled `rust-lld`, any `.rustup`
linker, and PATH-resolved LLVM tools outside that directory MUST NOT be used.
Changing any selected tool or resolved path invalidates the Phase 0 identity
and queue.

Compiler feature-test predicates that influence mechanically selected code are
also Phase 0 mechanical inputs. This includes `__has_attribute`,
`__has_builtin`, `__has_feature`, `__has_extension`, `__has_c_attribute`,
`__has_declspec_attribute`, and `__has_warning`. Each result MUST be obtained
by preprocessing a generated direct predicate probe with the frozen compiler,
architecture target, frozen configuration, and relevant original Kbuild command
flags. Compiler documentation, a host-only invocation, and an attribute parse
are not predicate evidence. The inventory retains the original and transformed
commands, raw stdout/stderr, executable and input hashes, timestamps, source
locations, and architecture; it is independently replayed and bound to the
Phase 0 identity. A changed predicate set/result/command/compiler/hash/target
or configuration invalidates the identity, manifests, and queue. Unlike
semantic interpretation, a mechanically relevant compiler-predicate result
MUST NOT remain `PENDING_REVIEW`.

## 2. Zero-difference contract

This is not a Linux-like redesign, a simplification exercise, a compatibility
facade, or a chance to replace Linux mechanisms with convenient Rust code.
Preserve Linux's:

- ABI, exported symbols, calling conventions, and layouts;
- algorithms, data structures, state machines, and asymptotic behavior;
- control flow, cleanup paths, operation ordering, and side effects;
- locking, barriers, atomics, RCU, refcounting, and lifetime rules;
- interrupt, preemption, per-CPU, scheduler, and asynchronous behavior;
- exact return values, partial-success behavior, flags, and `errno` values;
- allocation, stack, latency, boundedness, and scalability properties;
- compile-time behavior for the approved configuration union.

Build success, convenience, a smaller current machine, and idiomatic style never
override Linux semantics.

If exact behavior cannot be established from the pinned source, local headers,
callers, callees, Kconfig/Kbuild, and original tests, the affected semantic
record is `PENDING_REVIEW` during Phase 0 and the affected translation task is
`BLOCKED` only when its implementer/reviewers/applier reach that question.
Guessing is forbidden.

## 3. Authority order

When sources conflict, use this order:

1. the exact commit in `vendor/linux.SHA`;
2. the frozen x86_64/AArch64 configurations;
3. `rewrite/SCOPE.tsv` and its Kconfig/Kbuild evidence;
4. pinned Linux headers, callers, callees, generated definitions, and tests;
5. `rewrite/PORTING.md`, `rewrite/LIFETIMES.tsv`, and `rewrite/ABI.tsv`;
6. accepted findings from independent reviewers;
7. Rust style preferences.

Translation MUST NOT begin when the Linux revision is moving, mixed with
mainline backfills, or inconsistent with the checked-out `vendor/linux` tree.
No web page, model memory, or previous Lupos source may replace local pinned
upstream evidence.

## 4. Frozen configuration-derived subset

The authoritative subset is the union of checked-in configurations under:

```text
rewrite/configs/x86_64/
rewrite/configs/aarch64/
```

A Linux file or conditional branch is in scope only when mechanically generated
metadata records that it is selected by at least one approved configuration or
is a required transitive dependency of selected code. Mechanical selection is
the Phase 0 gate. Semantic call graphs, ownership, locking, RCU, refcount,
lifetime, ABI intent, and translation notes are progressive records and do not
block queue creation; unknown semantic values are recorded as `PENDING_REVIEW`.

The scope architect MUST finish the complete inventory before any translation
pipeline claims a file. During Phase 1, pipelines may not discover and silently
add work. An unlisted dependency blocks the task and reopens the scope gate.
All pipelines pause while a new scope and task-queue fingerprint is reviewed.

### 4.1 Source classes

Every considered Linux file MUST receive exactly one class in
`rewrite/SCOPE.tsv`:

| Class | Required treatment |
| --- | --- |
| `RUST_TRANSLATE` | Create a fresh path-preserving Rust translation under `src/`. |
| `LINUX_ARCH_ASM` | Preserve required x86_64/AArch64 assembly mechanically; do not redesign it. |
| `LINUX_DRIVER_OBJECT` | Do not translate. Build original Linux source later into its Kbuild object form and link or load it through the Linux-compatible ABI. |
| `ORACLE_ONLY` | Keep original KUnit, in-tree test, kselftest, test data, and vectors in `vendor/linux`. |
| `BUILD_METADATA` | Retain only the configuration/build metadata needed for the frozen subset. |
| `REFERENCE_ONLY` | Read for intent; do not import into `src/`. |
| `OUT_OF_SCOPE` | Do not translate, edit, stub, or claim. |

Driver ownership is determined by Kbuild target and subsystem role, not merely
by directory name. Driver-owned code in `drivers/`, `sound/`, architecture
platform directories, or elsewhere remains original Linux C/assembly objects.

## 5. Linux-shaped fresh source tree

The new Rust source tree mirrors relevant Linux paths:

```text
vendor/linux/kernel/sched/core.c       -> src/kernel/sched/core.rs
vendor/linux/arch/x86/kernel/apic/*.c  -> src/arch/x86/kernel/apic/*.rs
vendor/linux/arch/arm64/kernel/*.c     -> src/arch/arm64/kernel/*.rs
vendor/linux/include/linux/*.h         -> src/include/linux/*.rs
```

Rules:

- Map each selected implementation-bearing `.c` or `.h` file one-to-one where
  Rust permits it.
- Do not flatten directories or combine unrelated Linux files.
- Do not split one Linux file unless Rust makes it unavoidable and
  `rewrite/FILE_MAP.tsv` records every destination fragment.
- Do not create or update shared `mod.rs` indexes from parallel pipelines.
  Generate module indexes deterministically only after all file tasks are
  `DONE` and before the separate compile phase.
- Do not mirror Linux documentation, original tests, or driver sources under
  `src/`.
- Retain SPDX identifiers and relevant upstream copyright notices.

Every translated file begins with immutable provenance only:

```rust
// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: kernel/example.c
//! linux-revision: <exact SHA from vendor/linux.SHA>
//! architectures: x86_64,aarch64
//! rewrite-task: <stable task id>
```

Do not place mutable claims such as `complete`, `FULL`, `parity`, `tested`, or
`working` in source headers.

## 6. Phase 0 artifacts and complete task inventory

Before the first file is claimed, Phase 0 MUST create and review:

- `vendor/linux.SHA` — one exact commit;
- `rewrite/configs/**` — frozen x86_64 and AArch64 configurations;
- `rewrite/SCOPE.tsv` — all considered Linux files and their class/evidence;
- `rewrite/FILE_MAP.tsv` — exact source-to-destination mapping;
- `rewrite/SYMBOLS.tsv` — selected functions, types, statics, operative macros,
  and configuration branches for every `RUST_TRANSLATE` file;
- `rewrite/PORTING.md` — mechanical Linux C/assembly to Rust rules;
- `rewrite/LIFETIMES.tsv` — ownership/lifetime decisions derived from Linux;
- `rewrite/ABI.tsv` — layouts, alignment, linkage, symbols, and calling
  conventions;
- `rewrite/DRIVER_ABI.tsv` — contracts required by original Linux driver
  objects;
- `rewrite/BRANDING_ALLOWLIST.tsv` — every permitted Linux-to-Lupos name delta;
- `rewrite/TRANSLATION_TASKS.tsv` — the complete Phase 1 file queue;
- `rewrite/TRANSLATION_TASKS.sha256` — fingerprint of immutable queue fields;
- `rewrite/events.jsonl` — append-only workflow event log;
- `rewrite/logs/tasks/` — one evidence directory per task;
- `rewrite/BLOCKERS.tsv` — unresolved scope/source/lifetime/ABI questions.
- `rewrite/compiler-predicates/COMPILER_PREDICATES.tsv` and its fingerprint,
  commands, probes, raw output, and independent validation evidence.

Phase 0 also retains the mechanically generated Linux evidence under
`rewrite/metadata/`, including selected translation units, generated sources,
built-in/module object lists, source-to-object mappings, compile commands,
`.cmd`/depfile inventories, generated headers, and configuration selection
evidence. The exact filenames are recorded in the metadata manifest.

`SYMBOLS.tsv`, `LIFETIMES.tsv`, `ABI.tsv`, and `DRIVER_ABI.tsv` are initialized
from mechanically provable facts. A field that requires semantic interpretation
uses `PENDING_REVIEW`; it is completed by the implementer, both independent
reviewers, and the applier before that task can become `DONE`.

`rewrite/SCOPE.tsv` MUST expose at least:

```text
id	linux_path	destination_path	class	architectures	kconfig_evidence	kbuild_target	cluster	weight	risk	dependencies
```

All `RUST_TRANSLATE` rows are converted into task rows before Phase 1 begins.
No pipeline may start while the queue is incomplete or unfingerprinted.

The canonical Phase 0 identity is recorded in `rewrite/PHASE0_IDENTITY.tsv`
and its hash. It binds the Linux commit, both stabilized configuration hashes,
the toolchain hash, architecture invocation parameters, and extractor/schema
versions to every authoritative manifest and queue. Translation agents MUST
NOT modify the toolchain, frozen configurations, or Phase 0 identity.

### 6.1 Mechanical Phase 0 gate

Before queue creation, the original pinned Linux metadata workflow MUST produce
and validate the complete selected source inventory for both configurations.
At minimum it records each selected C/assembly/generated source, its object,
Kbuild target, built-in/module disposition, architecture membership, subsystem,
Kconfig evidence, and metadata evidence path. It also records generated
headers, compile commands, `.cmd` files, depfiles, and module/object lists when
the pinned Kbuild workflow emits them. Missing or contradictory mechanical
evidence blocks the scope gate. Semantic `PENDING_REVIEW` values do not.

## 7. Canonical translation queue

### 7.1 Required TSV schema

`rewrite/TRANSLATION_TASKS.tsv` is the canonical mutable snapshot. Its exact
header is:

```text
id	path	created_at	work_started_at	done_at	status	linux_path	architectures	cluster	weight	risk	dependencies	recommended_implementer	pipeline_id	attempt	lease_owner	lease_expires_at	implement_done_at	review_started_at	review_1_done_at	review_2_done_at	apply_started_at	updated_at	resume_status	last_error
```

Column rules:

- `id` is stable and unique for the Linux-path/destination-path pair.
- `path` is the destination Rust path and is unique across the queue.
- `created_at` is written when the complete queue is generated.
- `work_started_at` is written exactly once, on the first successful claim.
- `done_at` is empty until the final applier closes the file.
- `status` is one of the states in §7.3.
- `linux_path` resolves inside the pinned `vendor/linux` tree.
- `architectures` is a comma-separated subset of `common,x86_64,aarch64`.
- `dependencies` is a semicolon-separated list of task IDs.
- `weight` is a deterministic scheduling estimate, not a completion claim.
- `risk` is `low`, `medium`, or `high` and controls model escalation.
- `recommended_implementer` is `luna` or `spark`; it is advisory and every
  actual model invocation is also logged in `events.jsonl`.
- all timestamps use UTC RFC 3339 with milliseconds and a `Z` suffix.

The immutable columns are:

```text
id path created_at linux_path architectures cluster weight risk dependencies recommended_implementer
```

After the queue fingerprint is written, only status, lease, attempt, error, and
timestamp fields may change. Rows may not be appended, removed, reordered, or
repurposed during Phase 1. The fingerprint also binds the queue to the exact
commit in `vendor/linux.SHA`; changing the pinned revision reopens Phase 0.

### 7.2 Atomic queue updates

Agents MUST NOT hand-edit the queue. Every mutation goes through the checked-in
queue tool, which MUST:

- take an OS-level exclusive lock;
- verify the current branch;
- verify the immutable-field fingerprint;
- read the complete TSV;
- validate the requested state transition;
- write a temporary file in the same directory;
- `fsync` and atomically replace the TSV;
- append the corresponding event while holding the same lock.

The reference command surface is:

```bash
python3 tools/rewrite_queue.py init --scope rewrite/SCOPE.tsv
python3 tools/rewrite_queue.py freeze
python3 tools/rewrite_queue.py claim --pipeline P01 --worker <id>
python3 tools/rewrite_queue.py mark-implemented --id <task> --pipeline P01
python3 tools/rewrite_queue.py start-review --id <task> --pipeline P01
python3 tools/rewrite_queue.py mark-review --id <task> --slot 1 --pipeline P01
python3 tools/rewrite_queue.py mark-review --id <task> --slot 2 --pipeline P01
python3 tools/rewrite_queue.py start-apply --id <task> --pipeline P01
python3 tools/rewrite_queue.py done --id <task> --pipeline P01
python3 tools/rewrite_queue.py block --id <task> --pipeline P01 --reason <text>
python3 tools/rewrite_queue.py pause --id <task> --pipeline <Pxx> --reason <text>
python3 tools/rewrite_queue.py resume --id <task> --pipeline <Pxx> --worker <id>
python3 tools/rewrite_queue.py stats
python3 tools/rewrite_queue.py verify
```

A pipeline may hold at most one unresolved task. `claim` MUST refuse a second
task while that pipeline owns an active or `PAUSED` row. Only `DONE`,
`BLOCKED`, or an explicit `requeue` releases the pipeline for fresh work.

### 7.3 Status vocabulary

Use only these Phase 1 states:

| State | Meaning |
| --- | --- |
| `TODO` | Inventoried and ready when dependencies allow. |
| `IN_PROGRESS` | Exclusively leased to one pipeline; implementation is active. |
| `IMPLEMENTED` | Candidate written; no independent source review is complete. |
| `REVIEWING` | Two independent adversarial reviews are in progress or awaiting completion. |
| `APPLYING` | Both reviews are complete and the applier is resolving them. |
| `DONE` | Final source was rechecked by the applier; both review reports and a resolution exist. It has not been compiled or tested. |
| `BLOCKED` | Exact source/scope/ABI/lifetime behavior cannot be established. No placeholder is allowed. |
| `PAUSED` | An active stage stopped cleanly because of quota or interruption. `resume_status` preserves the exact stage and the task must be explicitly resumed. |

The normal transition is:

```text
TODO -> IN_PROGRESS -> IMPLEMENTED -> REVIEWING -> APPLYING -> DONE
```

`DONE` is the burn-chart event. It means only **source translation pipeline
complete**. It does not mean compiled, linked, booted, tested, compatible, or
parity-proven.

### 7.4 Event and evidence logs

Every stage start, finish, block, pause, retry, lease renewal, and completion
MUST append one JSON object to `rewrite/events.jsonl` containing at least:

```text
ts phase task_id path pipeline_id role event from_status to_status model reasoning_effort attempt detail
```

Each task uses:

```text
rewrite/logs/tasks/<id>/
  implementation.md
  candidate.diff
  parity-review.md
  rust-review.md
  resolution.md
```

The queue tool MUST refuse `DONE` unless all five files exist. Logs are evidence,
not implementation. Do not put source code copies in them beyond focused diff
snippets needed to explain findings. `IMPLEMENTED` and `DONE` also require a
non-empty destination file with exact task/source/revision/architecture
provenance and no `todo!`, `unimplemented!`, or Rust test configuration.

## 8. Quota-aware pipeline topology

The Bun-style idea is preserved—one implementer, two independent reviewers, one
applier—but concurrency is bounded for a personal Codex account.

### 8.1 Deterministic scheduler, not an LLM dispatcher

Task inventory, dependency checks, weighted selection, leases, timestamps, and
status updates are deterministic local tooling. Do not spend model tokens asking
an agent to choose the next row or count progress.

The scheduler claims only frozen `RUST_TRANSLATE` tasks whose dependencies are
`DONE`. It selects by dependency readiness, then highest remaining weight, then
higher review risk, then stable task ID. Atomic claims prevent duplicate work.

### 8.2 Default concurrency

Default operating limits:

```text
active file pipelines: 2
active task per pipeline: 1
maximum spawned subagents in one coordinator session: 4
reviewers per task: 2, concurrently
```

The queue permits only `P01` and `P02`; do not create additional pipeline IDs.
This is an intentional quota boundary for the personal-account experiment, not
a throughput target copied from Bun's 64-agent peak.

A pipeline must finish or block its current file before claiming another. A
`PAUSED` task retains the pipeline reservation until it is resumed, blocked, or
requeued, so that pipeline cannot claim fresh work in the meantime. Do not
prefetch tasks into model contexts.

Close completed spawned-agent threads as soon as their required artifact has
been captured. Completed implementers and reviewers must not remain open and
consume the four-thread cap while a later stage starts.

When a usage warning or interruption is imminent:

1. stop editing at a coherent point;
2. write the current task evidence;
3. transition it to `PAUSED` with a concrete reason and its owning pipeline;
4. leave the destination file, stage timestamps, and logs intact;
5. later use `resume` to restore the exact saved stage without repeating completed work;
6. do not let another pipeline claim it while it remains `PAUSED`.

## 9. Codex roles and model policy

Project-scoped custom agents live under `.codex/agents/`. Role separation is
mandatory even when roles use the same model family.

| Role | Default model | Effort | Purpose |
| --- | --- | --- | --- |
| scope architect | `gpt-5.6-sol` | `xhigh` | One-time pinned-source/config scope, mapping, ABI, and lifetime decisions. |
| pipeline coordinator | `gpt-5.6-terra` | `medium` | Runs the deterministic queue commands and spawns role-isolated agents; it does not translate. |
| implementer | `gpt-5.6-luna` | `medium` | Fast mechanical translation of exactly one leased file. |
| optional Spark implementer | `gpt-5.3-codex-spark` | `medium` | Low-risk, well-specified files only when the model is available and its separate preview limit has capacity. |
| parity reviewer | `gpt-5.6-terra` | `high` | Adversarial comparison with the pinned Linux implementation. |
| Rust reviewer | `gpt-5.6-terra` | `high` | Adversarial ownership, unsafe, FFI, layout, and Rust-semantics review. |
| applier | `gpt-5.6-terra` | `high` | Reopens Linux and resolves both reviews. |
| deep adjudicator / phase gatekeeper | `gpt-5.6-sol` | `xhigh` | High-risk conflicts, blockers, and whole-phase closure only. |

The implementer always uses less reasoning than reviewers and the applier.
Lower implementer effort never lowers the acceptance standard.

### 9.1 Spark policy

Spark is optional, never mandatory. It may be used only for `risk=low` tasks
with complete symbol/lifetime/ABI guidance. Before broad use, run a small trial
with matched Luna tasks and compare:

- elapsed implementation time;
- reviewer finding count and severity;
- rework/block rate;
- missing-symbol rate.

Record the actual model and effort in every event. Disable Spark when it causes
more omissions or rework, when its separate preview limit is exhausted, or when
it is unavailable. Fall back to Luna without creating a duplicate task.

Do not use Spark for high-risk concurrency, RCU, scheduler, interrupt, memory
management, ABI adjudication, final application, or phase-gate decisions.

### 9.2 Context separation

- The implementer does not review its own work.
- Reviewers run in separate contexts from the implementer and from each other.
- Reviewers receive the pinned Linux file, frozen manifests, relevant local
  headers/callers/callees, and the candidate diff. They do not receive the
  implementer's private rationale.
- Reviewers are source-read-only and assume the candidate is wrong; each may
  write only its assigned task report under `rewrite/logs/tasks/<id>/`.
- The applier receives the original source, candidate, both review reports, and
  frozen guidance; it independently adjudicates findings.
- A semantic conflict between reviewers escalates to the deep adjudicator or
  becomes `BLOCKED`.

## 10. Per-file pipeline

For each claimed row:

```text
atomic claim
    |
    v
implementer writes one fresh destination file
    |
    v
candidate snapshot + IMPLEMENTED
    |
    v
parity reviewer  ||  Rust reviewer
    |                    |
    +---------+----------+
              v
high-effort applier resolves every finding
              |
      +-------+-------+
      |               |
      v               v
    DONE        BLOCKED / PAUSED
```

### 10.1 Implementer

The implementer MUST:

1. verify task ID, lease, branch, Linux SHA, source path, destination path,
   architecture membership, selected symbols, and conditions;
2. read the complete Linux source file;
3. read every local pinned header, macro definition, type, caller, callee,
   Kconfig, and Kbuild rule needed to understand it;
4. translate every selected symbol and branch into the fresh destination file;
5. preserve control flow, cleanup, locking, ordering, errors, side effects,
   complexity, and ABI before considering style;
6. use safe Rust where exact semantics permit and minimal documented `unsafe`
   where the kernel contract requires it;
7. write `implementation.md` and `candidate.diff`;
8. transition the task to `IMPLEMENTED`;
9. block rather than guess.

The implementer MUST NOT:

- read historical Lupos Rust source;
- edit another task's destination;
- edit shared module indexes or global manifests by hand;
- add stubs, placeholders, fake success, hardcoded computed state, or future
  work comments in place of implementation;
- add Rust tests or copy Linux tests;
- port a driver;
- compile, format, link, execute, test, boot, debug, or benchmark;
- approve its own output.

### 10.2 Parity reviewer

The parity reviewer MUST compare the candidate exhaustively with the pinned
Linux source and selected inventory, including:

- all functions, types, statics, operative macros, and selected branches;
- algorithms, state machines, and data structures;
- success, retry, cleanup, and error paths;
- exact values, widths, signs, overflow, flags, masks, and errno;
- lock order, interrupt/preemption state, atomics, barriers, RCU, refcounts,
  wait/work queues, callbacks, and destruction order;
- stack/allocation/boundedness/scalability behavior;
- linkage, visibility, symbols, calling conventions, layouts, and Kconfig
  behavior;
- branding changes against the allowlist;
- omissions disguised as traits, wrappers, plans, reports, constants, or mocks.

Every finding names the Linux symbol and local evidence. The reviewer may write
only the leased task's `parity-review.md`; it MUST NOT edit source or any other
file, and it records review completion atomically.

### 10.3 Rust reviewer

The Rust reviewer MUST inspect:

- ownership and borrow duration against actual Linux lifetimes;
- pointer provenance, aliasing, pinning, and interior mutability;
- `Send`/`Sync` and cross-CPU access;
- `Drop` timing across callbacks, interrupts, work queues, RCU, and refcounts;
- `#[repr(C)]`, alignment, packing, unions, bitfields, endian behavior, and FFI;
- casts, truncation, sign extension, wrapping, shifts, C promotions, and pointer
  arithmetic;
- eager/lazy evaluation and debug/release semantic differences;
- panic, allocation failure, bounds-check, and unwind behavior;
- necessity and scope of every `unsafe` block.

It rejects idiomatic substitutions that change Linux behavior. It may write
only the leased task's `rust-review.md`; it MUST NOT edit source or any other
file, and it records completion atomically.

### 10.4 Applier

The applier MUST:

1. reopen the complete pinned Linux file and relevant context;
2. resolve every finding or disprove it with specific upstream evidence;
3. implement missing logic rather than documenting it as future work;
4. preserve the frozen task scope and destination path;
5. avoid introducing a new unreviewed design;
6. write `resolution.md` with one disposition per finding;
7. mark `DONE` only when no finding remains and all evidence files exist;
8. mark `BLOCKED` when exact parity cannot be established.

Before `DONE`, the applier also closes every `PENDING_REVIEW` semantic record
for the task, including symbols, ABI intent, ownership/lifetime, locking/RCU,
refcounting, and semantic dependencies. Phase 0 pending values are not an
approval or a substitute for this review.

The applier MUST NOT compile, format, link, run, test, benchmark, add tests, add
stubs, port drivers, or weaken Linux behavior.

## 11. Translation-only hard gate

The trusted project-local `.codex/rules/rewrite-safety.rules` is an additional
mechanical guard against direct compiler/runtime commands, destructive Git, and
historical-source recovery. Rules do not replace this protocol and may not cover
every possible shell wrapper; agents remain responsible for the complete ban.


The rule file is Phase-1-specific. After the whole-subset gate is approved, a
human—not an agent—must move it outside `.codex/rules/`, install the later
compile-workflow guardrails, and restart Codex. No translation agent may disable
or bypass its own command policy.

Phase 1 performs source translation and source review only.

Allowed:

- read-only inspection/search of pinned local source;
- deterministic scope and queue tooling;
- editing the one leased destination file;
- writing the task's unique evidence files;
- read-only `git status` and focused `git diff -- <leased-path>`;
- atomic queue/event updates.

Forbidden directly or through wrappers:

```text
cargo build
cargo check
cargo test
cargo fmt
cargo clippy
rustc
make
ninja
cmake
cc / gcc / clang
ld / lld
objcopy / objdump as validation
qemu-system-*
gdb / lldb
KUnit
kselftest
benchmarks
boot or userspace commands against Lupos
```

Do not use compiler errors as a Phase 1 work queue. The expected state is that
none of the fresh source works yet. As in the Bun rewrite workflow, source is
translated and adversarially reviewed first; compilation becomes a separate
later workflow only after every translation task is `DONE`.

## 12. Rust translation rules

Use this priority:

1. exact Linux semantics and ABI;
2. Rust soundness and explicit unsafe boundaries;
3. idiomatic Rust where it changes neither item 1 nor 2.

### 12.1 Mechanism is part of behavior

Preserve intrusive lists, rbtrees, xarrays, radix trees, per-CPU storage, wait
queues, work queues, RCU, seqlocks, refcounts, interrupt-driven paths, and state
machines. Do not replace them with convenient `Vec`, maps, fixed arrays,
polling, mock-only traits, or local protocols.

Reject on sight unless Linux does the same:

- a constant where Linux computes state;
- an errno stub where Linux performs work;
- a trait with no production backend;
- a `Plan`/`Report` object that omits side effects;
- constants/structs/helpers standing in for the operative file;
- a different container, algorithm, lock protocol, or complexity class;
- a large by-value copy where Linux moves pointers/list links;
- polling where Linux is interrupt-driven;
- simplification justified by current hardware limits;
- a paragraph-long workaround comment instead of a faithful port.

### 12.2 Unsafe

Use safe Rust when it can express the exact contract. Every `unsafe` block MUST
be minimal and include a local `// SAFETY:` comment naming the invariant and who
owns it. Every `unsafe fn` documents caller obligations. Do not create Rust
references whose exclusivity or lifetime is stronger than Linux guarantees.

### 12.3 Layout, linkage, errors, and cleanup

- Use `#[repr(C)]`, explicit widths, alignment, packing, unions, and exact
  exported names where required.
- Preserve padding and reserved fields.
- Preserve exact error signs, retry behavior, and partial success.
- Do not use `unwrap`, `expect`, or panic for recoverable Linux paths.
- RAII and `Drop` are allowed only when cleanup occurs at the same point and
  under the same synchronization as Linux.
- Preserve evaluation order and side effects; do not hide them in assertions,
  eager fallback expressions, or unconsumed iterators.

## 13. Branding

The Linux-to-Lupos name change is allowed only in
`rewrite/BRANDING_ALLOWLIST.tsv`. There is no global replacement.

Preserve Linux names by default, including UAPI names, exported symbols,
`CONFIG_*`, magic values, filesystem/protocol identifiers, driver KAPI names,
test names, and expected values.

## 14. No Rust unit tests

The rewritten kernel contains zero project-authored Rust unit tests. Do not add:

- `#[test]` or `#[cfg(test)]` modules;
- mock-only parity tests;
- copied/translated KUnit tests;
- `include_str!` assertions over `vendor/linux`;
- source-text pins presented as behavioral evidence;
- replacement vectors or expectations.

Original Linux tests are built later from the pinned Linux tree. A minimal
external harness may be added outside `src/` only to link, launch, and collect
those original tests without changing their logic or expected behavior.

## 15. Original Linux drivers remain objects

Drivers are not translated. For each selected driver:

1. classify it `LINUX_DRIVER_OBJECT`;
2. record required core ABI in `rewrite/DRIVER_ABI.tsv`;
3. expose the same Linux-facing contracts from the Rust core;
4. compile the original Linux driver source only in the later build phase;
5. link built-in objects or load `.ko` files according to the frozen config;
6. never create a Rust shell or fake driver in its place.

A later driver compile/link failure is a core ABI or build-integration task, not
permission to port the driver.

## 16. Git and shared-worktree safety

During Phase 1, the recommended personal-machine layout is one shared worktree
on `feat/bun-like-rewrite-test` with two pipelines. This avoids disk-heavy
worktrees while remaining safe because:

- the queue grants exclusive destination-file leases;
- workers never edit shared indexes/manifests directly;
- queue/event writes are locked and atomic;
- worker agents run no Git mutations.

Workers MUST NOT run `git stash`, `git reset`, `git clean`, checkout, restore,
rebase, merge, commit, push, or any command that discards or combines work.

A single human or dedicated integration process may commit completed files and
their evidence serially on the required branch. Never run concurrent commits.
Do not mix unrelated or non-`DONE` files into a completion commit.

## 17. Later phase queues

Only after every translation task is `DONE` and the phase gatekeeper verifies
the complete frozen queue may Phase 2 begin.

Later phases use separate TSV queues with the same base timestamp fields and the
same append-only event log:

- `rewrite/COMPILE_TASKS.tsv` — compiler/linker errors grouped by owning file or
  dependency cluster;
- `rewrite/TEST_TASKS.tsv` — failures from original Linux tests;
- `rewrite/RUNTIME_TASKS.tsv` — boot, ABI, compatibility, and performance
  failures.

### Phase 2 — compile and link

Generate compiler tasks only after one whole-subset compile attempt. Fix each
with one implementer/fixer, two reviewers, and one applier. Never use stubs,
deleted paths, altered ABI, or substitute algorithms to make the build pass.
Compile original Linux driver objects under the frozen configurations and link
both architectures.

### Phase 3 — original Linux tests

Build original KUnit/in-tree unit sources and kselftests from the pinned Linux
tree against Lupos. Preserve original test source, names, vectors, expected
values, and skip rules. Do not weaken or replace a failing test.

### Phase 4 — runtime and performance

Boot both approved architectures, run applicable original userspace/selftests,
compare Linux-visible behavior, debug only failures surviving earlier gates,
and retain exact commands/logs/traces/raw benchmark samples.

## 18. Progress charts and reporting

The canonical completion curve is derived from `done_at` or `event="done"`, not
from line counts, commits, or source headers.

Generate charts with:

```bash
python3 tools/plot_translation_burn.py \
  --queue rewrite/TRANSLATION_TASKS.tsv \
  --events rewrite/events.jsonl \
  --out-dir rewrite/plots
```

Required outputs:

- cumulative files `DONE` over time;
- files completed per hour;
- cumulative weight completed over time;
- machine-readable hourly metrics;
- per-attempt implementation, review, apply, and end-to-end durations, including
  archived/requeued attempts;
- per-model implementation counts, durations, blockers, pauses, review-report
  counts, and requeue summaries;
- a machine-readable JSON summary.

`translation_task_durations.tsv` and `translation_model_performance.tsv` are the
canonical evidence for comparing eligible implementer models. Raw speed alone
is insufficient: compare duration with reviewer findings, requeues, blockers,
pauses, and missing-symbol failures before changing the default implementer.
Attempt timing is derived from the append-only event log so retries are measured
separately rather than merged into one misleading wall-clock interval.

Every handoff reports:

- total tasks and status counts;
- completed count and weight;
- active pipelines and task IDs;
- blocked/paused tasks and reasons;
- per-model implementation counts and review-rejection rates when available;
- exact queue fingerprint and event-log path;
- no build/test claim during Phase 1.

## 19. Automatic rejection rules

Reviewers and appliers MUST reject candidates containing:

- historical Lupos translation source or copied old code;
- `todo!()`, `unimplemented!()`, placeholder panics, or fake success;
- hardcoded state where Linux computes or queries it;
- Rust tests, copied Linux tests, or source-text assertions;
- partial shells presented as a whole-file translation;
- a Rust driver rewrite;
- out-of-scope architecture/feature work;
- unauthorized branding;
- changed ABI names, layouts, flags, errno, or calling conventions;
- mock-only seams or convenient replacement mechanisms;
- unexplained/broad unsafe;
- compiler-driven changes during Phase 1;
- manual queue edits or missing timestamp events;
- multiple active tasks assigned to one pipeline;
- a `DONE` transition without both reviews and an applier resolution.

## 20. Phase 1 completion gate

Do not permit the first build until all are true:

- the branch is `feat/bun-like-rewrite-test`;
- no previous translated Rust kernel source was used;
- one Linux SHA and the x86_64/AArch64 config union are frozen;
- every selected file has one scope class and exact path mapping;
- every `RUST_TRANSLATE` file was listed in the TSV before work began;
- the immutable queue fingerprint verifies;
- every task is `DONE`; none is `TODO`, active, `BLOCKED`, or `PAUSED`;
- every task has implementation evidence, two independent reviews, and one
  resolution;
- every selected symbol and branch has a final mapping;
- every Phase 0 mechanical metadata record is complete and auditable;
- no `PENDING_REVIEW` semantic record remains for a `DONE` task;
- every unsafe boundary is documented;
- every selected driver remains original Linux source/object;
- no Linux docs/tests/drivers were copied into `src/`;
- the Rust kernel contains zero project-authored unit tests;
- only allowlisted branding differs intentionally;
- no compiler, linker, formatter, test, emulator, debugger, or benchmark ran in
  Phase 1;
- progress logs and charts can be regenerated from checked-in evidence.

The governing rule is: **inventory every file first, atomically burn one file
per pipeline through implement/review/review/apply, mark it `DONE`, and compile
nothing until the entire frozen queue is done.**

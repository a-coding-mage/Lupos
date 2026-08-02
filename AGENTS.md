# Lupos agent rules

## Non-negotiable parity rules

- The current target is the generic x86_64 configuration.
- `vendor/linux` is the source of truth. Lupos ABI and observable behavior must
  exactly match the corresponding Linux implementation.
- Keep implementation structure one-to-one with the corresponding
  `vendor/linux` C source whenever Rust can express it. A Rust-specific design
  is allowed only when a faithful translation is impossible; document the
  reason and prove equivalent behavior.
- Drivers are built from the original Linux C source and loaded by Lupos. Do
  not replace them with Rust rewrites.
- Before editing a kernel file, locate and read its `vendor/linux` equivalent.
  After editing it, compare the relevant control flow, constants, layouts,
  errors, ordering, locking, and side effects again.
- **Port Linux's implementation; never design a replacement.** If Linux solves
  the problem, the only acceptable implementation is a translation of Linux's
  data structures and algorithm. "Write it from scratch in Rust because it is
  simpler/easier/bounded" is not permitted, even when the result passes tests.
  Matching observable behavior is not sufficient — the mechanism must match too,
  because Linux's mechanism is what carries its performance and its
  memory/stack/latency guarantees. See "Fall back to Linux by default" below.
- Always report the truth. Never claim parity, a passing test, a benchmark
  result, or a root cause without evidence.
- Agents must not create branches or commit code without explicit approval.
- If a task has independent changes across multiple kernel areas, delegate the
  independent areas to focused sub-agents and evaluate their work as it
  arrives. Sub-agents must not run builds; the main agent owns all builds,
  integration, and final validation.

## Fall back to Linux by default

Lupos exists to be Linux, not to be an alternative kernel that behaves like it.
Whenever you are about to write a mechanism, stop and find Linux's first.

### The rule

1. Before writing any new mechanism, locate the `vendor/linux` equivalent and
   name it in a comment (`Ref:` / `linux-source:`). If you cannot find one, say
   so explicitly in the code and the handoff — do not silently invent.
2. Port Linux's **data structure and algorithm**, not just its outward
   behavior. `llist`, `list_head`, `rbtree`, `xarray`, `radix tree`, per-CPU
   areas, RCU-protected lists, and refcounts are part of the contract.
3. Prefer the Linux code path even when it looks like overkill for Lupos's
   current scale. Linux's structures are what make its costs bounded; a
   "simpler" substitute usually moves the cost somewhere invisible.
4. A Lupos-specific design requires **all** of: a documented reason a faithful
   translation is impossible, evidence of equivalent behavior, and a
   `linux-deviation:` marker in the file header naming the Linux function it
   replaces. Convenience, "Rust is different", and "we only support N CPUs" are
   not reasons.
5. When you find an existing Lupos-specific substitute while working nearby,
   report it as a finding even if it is not your bug.

### Known-bad substitution patterns

These have each caused a real, expensive Lupos defect. Treat them as defects on
sight:

- **Fixed-size arrays replacing Linux's intrusive lists.** Linux moves lists by
  pointer in O(1); a fixed array forces an O(n) copy, and a by-value move of a
  large array allocates a stack temporary of that size. The release kernel
  stack is Linux's x86-64 `THREAD_SIZE` (16 KiB, `KTHREAD_STACK_ORDER = 2`), so
  an 8 KiB temporary is half the stack. This is exactly how
  `begin_task_struct_rcu_release()`'s `core::ptr::swap` on
  `[*mut TaskStruct; 1024]` produced a `#DF` that then wedged every other CPU
  in the TLB shootdown wait. Linux's `rcu_do_batch()` moves the callback list
  by pointer (`rcu_cblist_flush_enqueue()`).
- **`ptr::swap` / `mem::swap` / `mem::replace` / by-value assignment of large
  arrays or large structs.** Every one allocates a full-size stack temporary.
  Budget against 16 KiB, not against what fits.
- **Static `MAX_*` matrices replacing Linux's dynamic per-CPU queues.** The TLB
  shootdown `[source][target]` slot matrix replaces
  `smp_call_function_many_cond()`'s per-CPU `llist` and loses its
  empty-to-non-empty IPI arming and whole-queue `llist_del_all()` drain.
- **Polling where Linux is interrupt-driven.** Any idle-loop pump that exists
  because a driver "has no IRQ path yet" is a latency bug with a deadline, not
  a design.
- **Reporting a constant where Linux computes a value.** `/proc` fields that
  return a fixed answer (for example a hardcoded `R (running)` task state) do
  not merely lose information — they actively mislead every later
  investigation.

### Stub and constant audit

A field, file, or syscall that returns a fixed value is a parity gap even when
nothing currently fails. When you touch such code, either implement Linux's
computation or record it in the handoff with the Linux function it should call.
Never let a stub be mistaken for working behavior in a report.

## Required investigation workflow

Make every issue reproducible and leave enough evidence for the next person to
continue without rediscovering the setup.

1. Reproduce the problem before changing code. Record the smallest reproducer,
   exact command, expected result, actual result, `.config`, relevant
   environment overrides, current revision, and dirty-worktree state. Do not
   erase or overwrite the user's existing changes.
2. Create `target/xtask/investigations/<issue>/` and retain raw evidence there:
   commands, serial logs, GDB transcripts, screenshots when relevant,
   benchmark samples, and a short `notes.md` containing the current hypothesis
   and eliminated causes. `target/` is ignored, so explicitly mention these
   paths in the handoff.
3. Capture enough environment information to reproduce tool-sensitive issues:

   ```bash
   git rev-parse HEAD
   git status --short
   rustc -Vv
   cargo -V
   qemu-system-x86_64 --version
   gdb --version
   uname -a
   ```

4. Read the matching Linux implementation and its tests before forming the
   fix. Trace inputs through the first point where Lupos diverges from Linux;
   do not stop at the first visible symptom.
5. For boot failures, keep the unique serial log printed by `cargo xtask` and
   run:

   ```bash
   cargo xtask boot-triage target/xtask/serial-<mode>-<run-id>.log
   ```

6. Reduce temporary instrumentation to focused probes. Remove noisy probes
   after the cause is proven, but retain the reproducer, regression test, and
   useful failure diagnostics.

## Bug investigation loop

**Parity first, debugging second.** Do not debug Lupos code that is not yet a
faithful translation of its `vendor/linux` counterpart. Divergent code cannot
be expected to behave or perform like Linux, so any conclusion drawn from it is
a conclusion about a Lupos invention, not about a real defect. Debugging a
non-parity implementation wastes the effort twice: once to explain behavior
that only exists because the code is different, and again after the code is
made correct and the explanation no longer applies.

Work these phases strictly in order.

### Phase 1 — Localize

Analyze until you can name where the issue most likely lives: a specific file
and code path, not a symptom. Use observation (serial logs, the QEMU monitor,
GDB, existing detectors) only to narrow the suspect area. Do not fix anything
in this phase, and do not form a root-cause theory from Lupos-only reasoning.

### Phase 2 — Bring that area to 1:1 parity

Before any debugging, read the matching `vendor/linux` source for every file
the suspect path touches and close every divergence you find:

- data structures and their invariants, not just function behavior;
- control flow, ordering, and locking;
- error paths, return values, and errno;
- constants, layouts, and ABI;
- the mechanism itself — see "Fall back to Linux by default".

Fix all of them, including divergences that plainly are not the reported bug.
Then re-test. Frequently the bug disappears here, because it *was* the
divergence. Record each divergence separately, with its Linux function, so the
list survives even when the symptom does not.

### Phase 3 — Debug only what survives

Only once the suspect path is a faithful translation, debug the remaining
behavior. Anything still failing is now a genuine defect worth deep
investigation, and its evidence is meaningful because the code under it matches
Linux.

### Phase 4 — Iterate

If the bug survives Phase 3, return to Phase 1 with what you learned and widen
the suspect area. Each iteration must eliminate candidates rather than restate
the hypothesis.

### Rules that apply across all phases

- Add or select a regression test covering each divergence so it cannot come
  back, following "Regression tests are part of every fix".
- Keep every parity fix that did not cause a regression, including fixes that
  turn out not to be the reported bug's cause. Never revert a correct fix
  merely because it was not the culprit; report it as a separate finding.
- Prefer this loop over waiting for an intermittent failure to reproduce.
  Observation only locates the suspect area; the comparison against
  `vendor/linux` is what identifies the defect.
- Lupos is largely machine-written, so assume a divergence exists and go find
  it rather than assuming the Rust side is already faithful. "I read it and it
  looked correct" is not a parity check; cite the Linux function you compared
  against.

## GDB-first debugging

Use GDB whenever the failing path can run under QEMU. This is mandatory for
crashes, panics, hangs, boot failures, corrupt state, and unexpected control
flow when a symbolized QEMU reproduction is possible; serial-log speculation
alone is not sufficient.

Start the smallest relevant mode in a symbolized debug build:

```bash
LUPOS_PROFILE=debug cargo xtask run --terminal --gdb
LUPOS_PROFILE=debug cargo xtask run --mode <mode> --gdb
LUPOS_PROFILE=debug cargo xtask run --gui --gdb
```

`--gdb` starts QEMU paused and exposes the stub on `localhost:1234`. In a
second terminal, run the exact `gdb <kernel-elf> -ex "target remote :1234"`
command printed by `xtask`. Set breakpoints or watchpoints before `continue`.
At minimum, capture the following when applicable:

```gdb
set pagination off
set logging file target/xtask/investigations/<issue>/gdb.txt
set logging enabled on
info registers
x/16i $pc
bt
thread apply all bt full
```

For a hang, interrupt GDB and collect all CPU backtraces, registers, the
current instruction stream, and relevant memory or lock state. For corruption,
prefer a watchpoint at the earliest known-good state. Break at both the Lupos
and corresponding Linux decision points when comparing behavior.

If GDB genuinely cannot be used (for example, the issue is host-only, the
failure disappears under the stub, or the required environment is not QEMU),
record the concrete reason in the investigation notes and use the closest
available evidence such as a core dump, `strace`, QMP capture, or serial trace.
"GDB would be inconvenient" is not a reason to skip it.

## Regression tests are part of every fix

- Add or select a regression test before implementing the fix and demonstrate
  that it fails for the reported behavior. Demonstrate that the same test
  passes after the fix.
- Prefer the original test from `vendor/linux`: KUnit, kselftest, LTP, the
  subsystem test tool, or the original reproducer. Port/adapt only the harness
  needed to run it on Lupos. Do not invent a local unit test when an upstream
  behavioral test exists.
- Every test-bearing Rust file must retain the repository's required
  `test-origin` provenance. Explain why a Lupos-specific test is necessary when
  no suitable Linux test exists.
- Match the test layer to the bug. Pure host logic may use a host unit test;
  syscalls, boot, interrupts, SMP, memory ordering, devices, modules, and
  userspace ABI behavior require the relevant QEMU/runtime gate. A
  source-text assertion is not runtime evidence and cannot be the sole
  regression test.
- Run the narrow failing test while iterating. Before handoff, always run
  `cargo xtask test`, the relevant `cargo xtask test --mode <mode>` (or
  `--boot`), and any original Linux test used as the oracle. Use
  `cargo xtask test --all` for cross-cutting, module, release, or broad ABI
  changes. Repeat timing-, SMP-, or race-sensitive tests enough to expose
  flakes.
- Never weaken, delete, ignore, or change a test's expected Linux behavior just
  to make a change pass. Report pre-existing and environment-blocked failures
  separately with their exact output.

## Performance regression gate

Every implementation change must identify the performance-sensitive path it
touches and use a relevant benchmark. If no benchmark exercises that path,
create one as part of the change. Documentation-only changes are exempt.

- Prefer the corresponding benchmark from `vendor/linux` or an upstream Linux
  test tool. When none exists, add the smallest reproducible benchmark that
  drives the real path, checks correctness, reports machine-readable samples,
  and documents why a Lupos-specific benchmark is necessary. Do not benchmark
  a mock in place of kernel behavior.
- Run a baseline before editing and the candidate after editing with the exact
  same workload, optimized profile, `.config`, QEMU version, accelerator, CPU
  model, machine, RAM, SMP count, disk image, host load policy, and warm-up
  state. Example fixed settings:

  ```bash
  LUPOS_PROFILE=release LUPOS_QEMU_ACCEL=tcg \
    LUPOS_QEMU_CPU=max LUPOS_QEMU_MEMORY=1024M \
    cargo xtask run --mode <benchmark-mode>
  ```

  Use KVM instead of TCG when the benchmark is intended to measure native CPU
  behavior, but never compare a KVM sample with a TCG sample.
- Collect raw per-iteration output under
  `target/xtask/benchmarks/<name>/{baseline,candidate}/`. Include warm-ups and
  enough measured repetitions to characterize noise (normally at least 10);
  compare median and tail latency or throughput, not a single wall-clock run.
- Correctness must pass before timing is considered. Boot time alone is not a
  proxy for the changed subsystem unless boot performance is the stated
  workload.
- A candidate must not be materially slower than baseline beyond the measured
  noise threshold. Do not hide a regression in averages. Any accepted
  performance trade-off requires explicit user approval and must be documented
  with raw numbers and rationale.
- Keep checked-in benchmarks runnable with one documented command and give
  their parsing/setup logic a correctness smoke test where practical. Avoid
  flaky timing thresholds in uncontrolled CI; preserve samples and evaluate
  thresholds in a controlled benchmark environment.

## Completion checklist

Before claiming an issue is complete, provide evidence for all of the
following:

- the original reproducer now passes;
- the regression test failed before the fix and passes after it;
- Linux source, ABI, errors, ordering, and behavior were re-compared;
- GDB was used, or the specific reason it could not be used was recorded;
- focused tests and the required broader gates passed;
- benchmark baseline and candidate samples show no material regression;
- investigation artifact paths and any remaining uncertainty are handed off.

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
- Always report the truth. Never claim parity, a passing test, a benchmark
  result, or a root cause without evidence.
- Agents must not create branches or commit code without explicit approval.
- If a task has independent changes across multiple kernel areas, delegate the
  independent areas to focused sub-agents and evaluate their work as it
  arrives. Sub-agents must not run builds; the main agent owns all builds,
  integration, and final validation.

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

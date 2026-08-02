Lupos agent rules

Mission and execution order

Lupos exists to reproduce Linux on the current generic x86_64 target. The taskis not to design a Linux-like kernel or to make a failing build pass by anyconvenient means. The task is to port Linux faithfully.

Every task must follow this order:

Preserve the user's worktree and capture the environment.

Define the exact parity scope.

Audit that complete scope against vendor/linux.

Close every parity gap in scope before debugging or diagnosing the build.

Build the parity-settled tree.

If the build, boot, test, or runtime path still fails, debug only thatsurviving failure.

Fix the failure by preserving Linux's ABI, data structures, algorithm,invariants, ordering, error behavior, side effects, and subsystem intent.

Rebuild, run the regression and compatibility gates, and measure the affectedperformance-sensitive path.

Report only claims supported by retained evidence.

A failure observed before parity is settled is only a locator. It is not yet avalid root cause. Do not spend time explaining, instrumenting, or repairingbehavior produced by a Lupos-specific divergence.

Non-negotiable parity rules

The current target is the generic x86_64 configuration.

vendor/linux is the source of truth. Lupos must match the correspondingLinux implementation's ABI, observable behavior, and implementationmechanism.

Port Linux's implementation; never design a replacement when Linux alreadyprovides one.

Preserve the purpose of the Linux code, not merely its output. Determine thatpurpose from the implementation, data-structure invariants, comments,callers, callees, tests, error paths, locking, memory ordering, lifetime andreference rules, and performance constraints.

Keep the Rust structure one-to-one with the corresponding vendor/linux Csource whenever Rust can express it.

A Rust-specific structure is permitted only when a faithful translation isgenuinely impossible. It requires:

a documented technical reason;

a linux-deviation: marker naming the Linux function or mechanism;

evidence of equivalent ABI and behavior;

evidence that Linux's ordering, boundedness, stack, latency, locking, andmemory-safety properties remain intact.

Convenience, reduced current scale, a fixed CPU limit, or "Rust is different"are not valid reasons for deviation.

Drivers are built from the original Linux C source and loaded by Lupos. Donot replace them with Rust rewrites.

Before editing any kernel file, locate and read its vendor/linuxcounterpart. After editing, compare the relevant Linux source again.

Always report the truth. Never claim parity, a passing build or test, abenchmark result, or a root cause without evidence.

Do not create branches or commit code without explicit user approval.

Do not overwrite, discard, reformat away, or silently absorb the user'sexisting changes.

Tasks must be atomic and deterministic. Do not create background work thatruns indefinitely.

Independent parity work across separate kernel areas may be delegated tofocused sub-agents. Sub-agents may inspect and edit only their assignedscope; they must not run builds. The main agent owns integration, all builds,all runtime execution, and final validation.

Define the parity scope before editing

The parity scope is not merely the file named in the task. It includes:

every Lupos file directly changed by the task;

the complete call path needed to compile or execute the affected behavior;

every shared type, constant, layout, macro equivalent, helper, error path,synchronization primitive, and ABI boundary used by that path;

the corresponding vendor/linux files and upstream tests;

any additional area implicated by a later build or runtime failure.

Create a parity ledger before editing. For each relevant item, record:

Lupos file, symbol, type, or ABI surface;

corresponding vendor/linux file and symbol;

whether the data structure matches;

whether the algorithm and control flow match;

whether constants, layouts, flags, errno, and return values match;

whether locking, barriers, ordering, ownership, reference handling, and sideeffects match;

the Linux or upstream test that proves the contract;

the divergence found and its correction status.

Do not claim that the whole kernel is at parity unless the whole kernel wasactually audited. The mandatory requirement is to close every gap in thecomplete task scope. Record unrelated out-of-scope gaps as findings. If abuild or runtime failure implicates a new area, expand the scope, audit thatarea, and close all of its parity gaps before resuming debugging.

Phase 0 — Preserve state and evidence

Before making changes, record:

git rev-parse HEAD
git status --short
rustc -Vv
cargo -V
qemu-system-x86_64 --version
gdb --version
uname -a

Also retain:

the exact task or reported symptom;

the exact command that exposed it, when one was provided;

expected and actual results;

.config and relevant environment overrides;

the current revision and dirty-worktree state.

Create:

target/xtask/investigations/<issue>/

Store the parity ledger, commands, compiler output, serial logs, GDBtranscripts, QMP or strace output when applicable, benchmark samples, and ashort notes.md there. Because target/ is ignored, mention the exact pathsin the handoff.

At this stage, an existing failure may be captured to identify the affectedscope. Do not yet diagnose its root cause or implement a failure-specificworkaround.

Phase 1 — Settle all vendor/linux parity gaps

This phase must finish before build debugging begins.

For every item in the parity scope, read the matching vendor/linux source,its surrounding definitions, relevant callers and callees, and its upstreamLinux tests. Compare and correct all of the following:

data structures and invariants;

algorithm and mechanism, not just final output;

control flow and operation ordering;

locking, interrupt state, preemption rules, atomics, and memory barriers;

lifetime, ownership, RCU, reference counting, and destruction order;

success and error paths;

return values and exact errno behavior;

constants, flags, masks, limits, and computed values;

C-compatible type layout, alignment, padding, signedness, width, and callingconvention;

syscall, ioctl, procfs, sysfs, module, driver, and userspace ABI;

externally visible side effects and their order;

stack, allocation, latency, scalability, and boundedness properties;

comments or tests that reveal why Linux uses that mechanism.

Fix every discovered gap in scope, including one that is clearly unrelated tothe originally reported symptom. A correct parity fix must not be revertedmerely because it was not the symptom's cause.

Each ported implementation must name its Linux source with the repository'saccepted Ref: or linux-source: convention. If no Linux equivalent can befound, say so explicitly in the source and handoff; never silently invent amechanism.

The Linux mechanism is part of the contract

Port Linux's data structure and algorithm. Matching visible results with asubstitute is insufficient. Linux mechanisms such as llist, list_head,enumerated state machines, rbtree, xarray, radix trees, per-CPU areas,RCU-protected lists, wait queues, work queues, refcounts, and interrupt-drivenpaths carry performance, ordering, stack, and latency guarantees.

Prefer the Linux path even when it appears excessive for Lupos's currentscale. A simpler replacement usually shifts cost or risk into an unmeasuredpath.

Known-bad substitution patterns

Treat these as parity defects on sight:

Fixed-size arrays replacing Linux intrusive lists. Linux can detach orsplice a list in O(1). An array commonly introduces O(n) copying and maycreate a large by-value stack temporary.

ptr::swap, mem::swap, mem::replace, or by-value assignment of largearrays or structures. Budget against Linux x86-64 THREAD_SIZE of 16 KiB.A full-size temporary can exhaust the kernel stack.

Static MAX_* matrices replacing Linux dynamic per-CPU queues. These canlose Linux's queue-drain, empty-to-non-empty notification, and scalabilitysemantics.

Polling where Linux is interrupt-driven. An idle-loop pump is a latencydefect, not a permanent implementation strategy.

A constant where Linux computes a value. A hardcoded procfs, sysfs,scheduler, memory, or task-state result is a parity gap and can mislead laterdebugging.

A local container replacing the Linux container. A Vec, array, map, orcustom queue is not equivalent merely because tests currently pass.

A local lock or atomic protocol replacing Linux ordering. Equivalentfinal values do not prove equivalent memory visibility or race behavior.

The historical begin_task_struct_rcu_release() failure is the model warning:using core::ptr::swap on [*mut TaskStruct; 1024] created a large stackcopy, caused #DF, and left other CPUs waiting in TLB shootdown. Linux'srcu_do_batch() instead moves the callback list by pointer throughrcu_cblist_flush_enqueue().

Stub and constant audit

A field, syscall, procfs entry, sysfs entry, helper, or file that returns afixed value where Linux computes one is a parity gap even when nothing fails.When such code is in scope, implement the Linux computation. When it is outsidescope, record it with the Linux function it should use. Never report a stub asworking behavior.

Parity phase exit criteria

Do not proceed to the build phase until:

every parity-ledger entry in scope is resolved or explicitly documented as ajustified linux-deviation:;

every changed file has been re-compared with its Linux counterpart;

the relevant ABI layouts and constants have explicit checks where practical;

an upstream regression test or the closest valid oracle has been selectedfor every corrected contract;

no known Lupos-specific substitute remains in the execution path beinghanded to the build.

Phase 2 — Build the parity-settled tree

Only now run the exact build command requested by the task or the repository'scanonical documented build command. Preserve the complete output in theinvestigation directory.

A successful build does not prove parity; it only allows validation tocontinue. A failed build does not permit a workaround that changes Linux's ABIor intent.

For each build error:

Identify the first real compiler, linker, layout, symbol, or generated-codefailure rather than reacting to downstream cascades.

Add the newly implicated files and symbols to the parity scope.

Stop build debugging and compare that expanded scope with vendor/linux.

Close every newly discovered parity gap there.

Re-run the same build command.

Repeat until the parity-settled tree builds or an external environmentblocker is proven with exact evidence.

When adapting C semantics to Rust, preserve Linux's ABI and intendedmechanism. Do not make the build pass by:

changing a public type, layout, calling convention, flag, or symbol contract;

deleting or bypassing code paths;

weakening type or bounds checks that represent a Linux invariant;

replacing a Linux algorithm with a simpler Rust implementation;

suppressing an error without proving it is spurious;

hardcoding a value Linux derives;

weakening or altering a test's expected Linux behavior.

Phase 3 — Reproduce and debug only surviving failures

After the parity-settled tree builds, run the smallest relevant boot, test, orruntime gate. If it passes, continue to the broader validation gates. If itfails, the surviving failure is now a valid debugging target.

Record the smallest reproducer, exact command, expected result, actual result,configuration, environment, and unique logs. Trace inputs to the first pointwhere actual execution diverges from the corresponding Linux decision orinvariant. Do not stop at the first visible symptom.

For boot failures, retain the unique serial log printed by cargo xtask andrun:

cargo xtask boot-triage target/xtask/serial-<mode>-<run-id>.log

Use focused probes only. Remove noisy temporary instrumentation after the causeis proven, while retaining the reproducer, useful diagnostics, and regressiontest.

GDB-first runtime debugging

Use GDB whenever the failing path can run under QEMU. It is mandatory forcrashes, panics, hangs, boot failures, corrupt state, and unexpected controlflow when a symbolized QEMU reproduction is possible. Serial-log speculationalone is not sufficient.

Start the smallest relevant symbolized debug mode:

LUPOS_PROFILE=debug cargo xtask run --terminal --gdb
LUPOS_PROFILE=debug cargo xtask run --mode <mode> --gdb
LUPOS_PROFILE=debug cargo xtask run --gui --gdb

--gdb starts QEMU paused and exposes the stub on localhost:1234. In asecond terminal, run the exact command printed by xtask:

gdb <kernel-elf> -ex "target remote :1234"

Set breakpoints or watchpoints before continue. Capture, when applicable:

set pagination off
set logging file target/xtask/investigations/<issue>/gdb.txt
set logging enabled on
info registers
x/16i $pc
bt
thread apply all bt full

For a hang, interrupt GDB and collect every CPU's backtrace, registers, currentinstructions, and relevant lock or memory state. For corruption, prefer awatchpoint at the earliest known-good state. Compare the Lupos execution pointwith the corresponding Linux decision point and invariant.

If GDB genuinely cannot be used—for example, the defect is host-only, vanishesunder the stub, or requires a non-QEMU environment—record the concrete reasonand use the closest available evidence, such as a core dump, strace, QMPcapture, or serial trace. Inconvenience is not a valid reason to skip GDB.

Phase 4 — Fix the surviving defect without violating Linux

A post-parity fix must still be a Linux-faithful correction. Before editing:

identify the violated Linux invariant or contract;

identify the corresponding Linux function, type, test, or call path;

explain why the parity-settled Rust translation still diverges in execution;

select or add the regression test that demonstrates the failure.

Then implement the smallest correction that restores Linux's behavior andmechanism. Preserve:

ABI and layout;

upstream algorithm and data structure;

return values and errno;

ordering, barriers, locks, and side effects;

ownership, lifetime, RCU, and refcount semantics;

stack and allocation bounds;

interrupt, preemption, and per-CPU behavior;

the subsystem purpose demonstrated by Linux's callers and tests.

Rebuild after each focused correction. If the new evidence implicates anotherarea, return to the parity phase for that expanded scope before continuing todebug.

Regression tests are part of every parity correction and defect fix

Select or add the regression oracle before implementing each correction.

Demonstrate that it fails against the relevant pre-correction behavior whenthat state is buildable and executable, and that it passes afterward.

Prefer the original Linux test from vendor/linux: KUnit, kselftest, LTP,the subsystem test tool, or the original reproducer. Port or adapt only theharness required to execute it on Lupos.

Do not invent a local unit test when an upstream behavioral test alreadydefines the contract.

Every test-bearing Rust file must retain the repository's requiredtest-origin provenance. Explain why a Lupos-specific test is necessary whenno suitable Linux test exists.

Match the test layer to the defect. Pure host logic may use a host unit test.Syscalls, boot, interrupts, SMP, memory ordering, devices, modules, anduserspace ABI require the relevant QEMU or runtime gate.

A source-text assertion is not runtime evidence and cannot be the soleregression test.

Never weaken, delete, ignore, or change a test's expected Linux behavior tomake a change pass.

Report pre-existing and environment-blocked failures separately with exactoutput.

While iterating, run the narrowest valid test. Before handoff, run:

cargo xtask test
cargo xtask test --mode <relevant-mode>

Use --boot where appropriate, run any original Linux test used as the oracle,and use:

cargo xtask test --all

for cross-cutting, module, release, or broad ABI changes. Repeat timing-, SMP-,or race-sensitive tests enough times to expose flakes.

Performance regression gate

Every implementation change must identify the performance-sensitive real pathit touches and exercise it with a relevant benchmark. Documentation-onlychanges are exempt.

Prefer the corresponding vendor/linux benchmark or an upstream Linux testtool. When none exists, add the smallest reproducible benchmark that:

drives the real kernel path rather than a mock;

checks correctness;

emits machine-readable samples;

documents why a Lupos-specific benchmark is necessary.

For a post-parity defect fix, the baseline is the parity-settled tree before thespecific fix, and the candidate is the corrected tree. A faster non-parityimplementation is never a valid reason to preserve a Linux divergence. When apre-correction state cannot build or execute, record that fact and compare thecandidate against the closest valid parity baseline and Linux's intendedcomplexity and boundedness guarantees.

Use the same workload, optimized profile, .config, QEMU version,accelerator, CPU model, machine, RAM, SMP count, disk image, host-load policy,and warm-up state. Example fixed settings:

LUPOS_PROFILE=release LUPOS_QEMU_ACCEL=tcg \
  LUPOS_QEMU_CPU=max LUPOS_QEMU_MEMORY=1024M \
  cargo xtask run --mode <benchmark-mode>

Use KVM instead of TCG when measuring native CPU behavior, but never compare aKVM sample with a TCG sample.

Store raw per-iteration output under:

target/xtask/benchmarks/<name>/baseline/
target/xtask/benchmarks/<name>/candidate/

Include warm-ups and enough measured repetitions to characterize noise,normally at least ten. Compare median and tail latency or throughput, not asingle wall-clock run. Correctness must pass before timing is considered. Boottime is not a proxy for a subsystem unless boot performance is the statedworkload.

The candidate must not be materially slower than the valid baseline beyond themeasured noise threshold. Do not hide regressions in averages. Any acceptedperformance trade-off requires explicit user approval and retained raw numbersand rationale.

Keep checked-in benchmarks runnable with one documented command. Wherepractical, give their setup and parsing logic a correctness smoke test. Avoidflaky timing thresholds in uncontrolled CI; preserve samples and evaluate themin a controlled environment.

Completion checklist

Do not claim completion until evidence proves all applicable items:

the exact task scope and any later expansions were recorded;

every parity gap in that scope was closed or documented with an approvedlinux-deviation: and proof;

every changed file was re-compared with its vendor/linux counterpart;

Linux's data structures, algorithm, invariants, ABI, layouts, constants,errors, ordering, locking, barriers, lifetimes, and side effects match;

the parity-settled tree builds successfully;

the original reproducer passes, when one exists;

each regression test failed before the relevant correction when demonstrableand passes afterward;

GDB was used for an applicable runtime failure, or the concrete reason itcould not be used was recorded;

focused tests and all required broader gates passed;

benchmark samples show no material regression against a valid paritybaseline;

raw evidence and investigation paths were handed off;

remaining uncertainty, environment blockers, pre-existing failures, andout-of-scope parity findings were reported explicitly;

no branch or commit was created without approval.

The final report must distinguish clearly between:

parity gaps corrected before the build;

build errors that survived the parity sweep and how they were fixed;

runtime or test defects that survived a successful build;

validation and benchmark evidence;

unresolved or out-of-scope findings.

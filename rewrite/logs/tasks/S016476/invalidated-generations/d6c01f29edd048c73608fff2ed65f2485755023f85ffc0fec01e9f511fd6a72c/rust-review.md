# Rust review — S016476

Role: Rust reviewer (independent, source-only)  
Pipeline: P01  
Task: `include/uapi/linux/wait.h` -> `src/include/uapi/linux/wait.rs`  
Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`  
Architectures reviewed: x86_64, aarch64

## Evidence inspected

- Complete pinned source: `vendor/linux/include/uapi/linux/wait.h:1-23`.
- Frozen scope and symbol records for S016476: selected as common, with two
  header-closure consumers per architecture (`kernel/exit.o` and
  `kernel/pid_namespace.o`); no ABI or lifetime rows exist because the source
  declares only object-like macros.
- Direct selected consumer semantics:
  `vendor/linux/kernel/pid_namespace.c:244` passes `__WALL` as the third
  argument to `kernel_wait4`; `vendor/linux/include/linux/sched/task.h:106`
  declares that parameter as `int`; `vendor/linux/kernel/exit.h:12-23` stores
  the resulting flags in `struct wait_opts::wo_flags`, also `int`.
  `vendor/linux/kernel/exit.c:1769-1824` accepts `int options`, validates and
  stores it; `kernel/exit.c:1880-1911` does the same for `kernel_wait4`.
  The frozen configurations enable `CONFIG_COMPAT` on both architectures, so
  the same signed `int options` ABI is also used by the selected compat wait
  entry points in `kernel/exit.c:1965-1984`.

## Finding R1 — High: all wait-option macros were exposed as `u32`, changing their signed API contract

`wait.rs:8-25` declares every wait-option constant as `u32`.  In the pinned
C header, the unsuffixed hexadecimal literals through `__WALL` have C `int`
type on the frozen 32-bit-`int` x86_64 and AArch64 targets:
`WNOHANG`, `WUNTRACED`, `WSTOPPED`, `WEXITED`, `WCONTINUED`, `WNOWAIT`,
`__WNOTHREAD`, and `__WALL`.  They are consumed as signed `int options` and
signed `int wo_flags` in the selected wait paths.  For example,
`pid_namespace.c:244` can pass C `__WALL` directly to the `int` argument, but
the Rust public `u32 __WALL` cannot be supplied to an equivalent `i32` API or
combined with its signed flag state without a non-source-equivalent cast at
each call site.

`__WCLONE` needs a separate, explicit decision: C `0x80000000` is an
unsuffixed hexadecimal `unsigned int` literal, and C converts it to the
signed `int` options/flags storage and performs the relevant usual arithmetic
conversions in expressions such as `wo_flags & __WCLONE`.  The candidate's
all-`u32` choice erases the documented/required signed interface of every
other flag and leaves no representation of that boundary.  This is material
to UAPI/core call compatibility and signed bitwise composition, even though
the bit patterns happen to match.

Required applier disposition: replace the undifferentiated `u32` public API
with a representation that preserves the selected `int` wait-options
contract, including the `0x80000000` / `__WCLONE` bit-pattern conversion
explicitly (for a signed Rust options type, `i32::MIN` is the matching
two's-complement bit pattern).  The resolution must state how the raw C
`unsigned int` literal type of `__WCLONE` is represented at the Rust boundary
and must preserve all mixed flag-mask operations without panic or accidental
checked conversion.

## Remaining semantic facts for closure

- `WSTOPPED` is an object-like alias whose expansion is exactly `WUNTRACED`;
  the candidate retains the equal numeric value.  All other listed names are
  object-like, side-effect-free integer constant macros.  There are no
  function-like/composite macros, allocation, errors, ownership, locking,
  RCU, refcount, layout, linkage, or calling-convention declarations in this
  file.
- `P_ALL`, `P_PID`, `P_PGID`, and `P_PIDFD` are ordinary C `int` literals and
  are correctly exposed as `i32` at `wait.rs:28-34`.
- `_UAPI_LINUX_WAIT_H` is solely a repeated-include guard.  It has no
  configuration predicate or runtime/UAPI value to export; Rust's unique
  module inclusion is the appropriate equivalent, provided the later
  deterministic module-index generation exposes this path exactly once.
- The header contains no architecture or Kconfig conditional branch.  The
  frozen header closure selects the same complete macro set for x86_64 and
  AArch64.  Provenance, SPDX expression, source path, revision, architecture
  union, and task ID in the candidate match the pinned/frozen records.

## Review result

Reject pending resolution of R1.  No compiler, formatter, test,
rust-analyzer, linker, or runtime tool was invoked or used as evidence.

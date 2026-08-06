# Application resolution — S016476

- Pipeline: `P01`
- Role/model/effort: applier / Terra / high
- Linux source: `include/uapi/linux/wait.h`
- Destination: `src/include/uapi/linux/wait.rs`
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (`x86_64`, `aarch64`)

## Frozen-input and scope recheck

The required branch is `feat/bun-like-rewrite-test`; the verified immutable
queue fingerprint is
`af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`.
S016476 is leased by P01 and is in `APPLYING`.  The frozen Phase 0 identity
binds the stated Linux revision, both frozen configurations, and the canonical
LLVM-19 metadata input.  `rewrite/SCOPE.tsv` records this as a common,
header-closure-selected `RUST_TRANSLATE` file, with `kernel/exit.o` as the
recorded direct consumer on both architectures; the frozen closure also
records `kernel/pid_namespace.o` as the second selected consumer.  Both frozen
configurations enable `CONFIG_COMPAT`.

The complete source header has only its include guard and thirteen object-like
integer macros: nine wait-option macros and four `waitid` selector macros. It
has no types, layouts, functions, Kconfig branch, ownership, allocation,
locking, RCU, refcount, ABI linkage, or lifetime operation. The guard is a C
repeated-include mechanism; Rust module inclusion supplies the corresponding
single definition and does not export an extra UAPI value.

## Review-finding dispositions

### P1 / R1 — resolved: signed options representation and `__WCLONE`

Both reviewers correctly rejected the uniform `u32` candidate. In the pinned
header, `WNOHANG`, `WUNTRACED`, `WSTOPPED`, `WEXITED`, `WCONTINUED`, `WNOWAIT`,
`__WNOTHREAD`, and `__WALL` are unsuffixed hexadecimal literals whose selected
targets use the 32-bit signed `int` representation. `WSTOPPED` remains the
source alias of `WUNTRACED`. `__WCLONE` is the distinct unsuffixed hexadecimal
`0x80000000` literal, whose C type is `unsigned int`.

Pinned `kernel/exit.h` defines `struct wait_opts::wo_flags` as `int`.
Pinned `kernel/exit.c` accepts `int options` in `kernel_waitid_prepare` and
`kernel_wait4`, validates bit masks containing all nine option values, stores
the value in `wo_flags`, and performs every selected flag operation on that
signed storage. Pinned `kernel/pid_namespace.c` passes `__WALL` to the signed
`int options` parameter of `kernel_wait4`. The selected compat wait entry
points likewise take this signed `int` ABI.

The applied representation therefore exposes each option name as `i32`, the
Rust type for the selected signed C `int` boundary. For the raw unsigned C
`__WCLONE` literal, `i32::MIN` explicitly denotes the same 32-bit
`0x80000000` bit pattern at that boundary. Rust signed bitwise `|`, `&`, and
`!` preserve the 32-bit pattern without a checked or panicking conversion, so
the selected mask validation, storage, and flag tests retain the C target's
effective operation. This is not a new flags wrapper or facade: every
source-defined UAPI name remains a public integer constant with its original
value and alias relationship.

The final source recheck also corrected `WSTOPPED` itself to `i32`; it remains
an alias expression of the signed `WUNTRACED` constant rather than an
independently duplicated literal.

### Selector macros — resolved

`P_ALL`, `P_PID`, `P_PGID`, and `P_PIDFD` remain public `i32` constants with
the exact source values `0`, `1`, `2`, and `3`. They are the source `int`
selectors used by the complete `kernel_waitid_prepare` switch, including the
`P_PIDFD` branch. No enum, remapping, or renamed ABI is introduced.

### Provenance and semantic-record closure — resolved

The destination retains the exact source path, frozen revision, common
architecture union, task ID, and SPDX expression. No task-specific row exists
in the frozen ABI, lifetime, driver-ABI, or blockers manifests; this follows
from the source's constants-only content. The task's previously pending symbol
records are closed by the above source mapping: the guard is a non-exported
inclusion mechanism, the nine option macros map to the documented `i32`
constants, and the four selectors map to their documented `i32` constants.
There are no remaining task-local ownership, lifetime, layout, linkage,
locking, or configuration decisions.

## Final source-only check

Reopened the complete pinned header, both review reports, direct wait-path
contexts, frozen configurations, scope/symbol/ABI/lifetime records, and the
candidate. The candidate now contains all thirteen source macro names, exact
numeric values, the `WSTOPPED` alias, and no additions beyond Rust-required
module treatment and documented signed-boundary representation. No compiler,
formatter, linker, test, runtime tool, rust-analyzer diagnostic, historical
Lupos source, shared module index, or unrelated source was used or changed.

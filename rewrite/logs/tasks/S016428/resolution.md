# Resolution — S016428 / P02 attempt 1

## P1 — `__KERNEL__` conditional API is erased

**Disposition: accepted; unresolved — recommend `BLOCKED`.**

Pinned source `vendor/linux/include/uapi/linux/tty_flags.h:42-53` and
`:84-95` make the ten `ASYNCB_*` internal positions and nine `ASYNC_*`
internal masks available only when `__KERNEL__` is *not* defined.  The frozen
kernel build context defines that macro: `vendor/linux/Makefile:612` sets
`KBUILD_CPPFLAGS := -D__KERNEL__`.  The selected header-closure records also
bind this header to kernel object consumers for both frozen architectures
(`rewrite/metadata/header_closure.tsv:7158` for aarch64 and `:11474` for
x86_64).  Therefore the candidate's unconditional Rust constants do not
model the selected kernel-side C interface.

No source-proven Rust configuration boundary exists in the frozen task
inputs.  The only checked-in package manifest (`Cargo.toml`) declares no
features or separate user-facing artifact; the current fresh Rust tree has no
established `#[cfg]` consumer-mode mechanism.  Adding a new feature, cfg name,
or second UAPI/core module would be a new unreviewed design outside this
file's frozen mapping, not a translation of a pinned selection mechanism.

Removing the nineteen constants would reproduce their kernel-side absence,
but it would still not translate the separate unconditional macro
`ASYNC_SUSPENDED` at pinned line 57.  That C macro name exists in both modes,
yet in kernel mode its expansion contains the deliberately undefined
`ASYNCB_SUSPENDED` token.  A Rust `const` requires a resolved operand and
would necessarily give the expression a usable `u32` value, as the candidate
does at `src/include/uapi/linux/tty_flags.rs:40`.  A Rust macro would require
a different invocation interface and still needs an unapproved consumer-mode
design; it is not an equivalent typed replacement.  No pinned source or
frozen Rust build metadata establishes a faithful representation of that
missing-operand compile-time behavior.

Consequently I made no source change.  Exact parity for the two guarded groups
and the kernel-mode `ASYNC_SUSPENDED` behavior cannot be established from
manual local source evidence.  The task must not advance to `DONE`; its queue
owner should use the queue tool to mark it `BLOCKED` with this reason.

## RUST-001 — `__KERNEL__` macro visibility was erased

**Disposition: accepted; unresolved — recommend `BLOCKED`.**

This finding is the same concrete interface defect as P1, independently
confirmed from the Rust side.  The candidate publishes all guarded names at
`tty_flags.rs:28-37` and `:69-77`, and gives `ASYNC_SUSPENDED` a resolved
kernel-side value at `:40`; pinned C provides neither property for a
`-D__KERNEL__` translation unit.  The local searches found no selected use of
these obsolete guarded identifiers beyond their definitions, but absence of a
current use is not authority to alter a frozen compile-time interface.

The required consumer-bound Rust configuration or split UAPI/core artifact is
not specified by the pinned source, frozen configurations, `SCOPE.tsv`,
`FILE_MAP.tsv`, `PORTING.md`, `ABI.tsv`, or `LIFETIMES.tsv`.  Creating one
would exceed the frozen task scope.  This review finding is therefore accepted
and remains unresolved for the same source-evidence blocker stated above.

No compiler, formatter, linker, test, runtime command, Git command, or
historical Rust source was used.  This report changes only task evidence; the
candidate and queue remain untouched.

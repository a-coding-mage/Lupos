# Parity review — S016427 / include/uapi/linux/tty.h

## Result: FINDINGS

### P001 — the selected UAPI include-guard macro is absent

`include/uapi/linux/tty.h:2-3,46` defines the C preprocessor contract
`_UAPI_LINUX_TTY_H` through `#ifndef`, `#define`, and the matching `#endif`.
The frozen `SYMBOLS.tsv` inventory selects both architecture instances of that
conditional and macro.  `src/include/uapi/linux/tty.rs` has no mechanism that
defines or tests this C macro, so repeated C inclusion and `#ifdef
_UAPI_LINUX_TTY_H` observe a different interface.  A Rust module item cannot
provide a C preprocessing definition.  No source-proven compatibility boundary
or retained UAPI-header mechanism is present in the candidate or frozen ABI
records.

Affected semantic records: the `selection_expression` records for
`_UAPI_LINUX_TTY_H` on `aarch64` and `x86_64`.

### P002 — all selected line-discipline macros are replaced by Rust-only items

`include/uapi/linux/tty.h:10-44` exports `N_TTY` through `N_CAN327` and
`NR_LDISCS` as object-like C macros.  The candidate reproduces their decimal
values as `pub const ...: i32`, but that changes the exported mechanism and
namespace: a C include receives textual macro substitution in its current C
integer-expression context, while a Rust `pub const` is a namespaced Rust item
and cannot be consumed by the original C callers.  The pinned source shows
`drivers/tty/tty_ldisc.c:47,62,144,189,195` using `NR_LDISCS` and `N_TTY` in
array bounds, comparisons, and return expressions.  The manifest records the
macros as selected operative macros for both frozen architectures, but provides
no completed ABI or adapter record establishing that a Rust-only `i32` item is
an exact replacement for the C UAPI macro interface.

This is not a value-only discrepancy: retaining 0 through 31 does not preserve
preprocessor availability, macro substitution, C identifier lookup, or the
unqualified UAPI namespace.  Exact parity requires a source-proven C-facing
header/bridge mechanism (and its ABI ownership) before these records can be
closed; none is present in the candidate or frozen direct records.

Affected semantic records: every `selection_expression` record for
`N_TTY` through `N_CAN327` and `NR_LDISCS`, for `aarch64` and `x86_64`.

No compiler, formatter, analyzer, test, or runtime command was used.

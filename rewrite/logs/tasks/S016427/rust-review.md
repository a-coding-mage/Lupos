# Rust review — S016427 / attempt 1 / P01

Reviewed only the pinned `vendor/linux/include/uapi/linux/tty.h`, the fresh
candidate snapshot, and frozen task records. No compiler, formatter, tests,
historical Lupos source, or implementation rationale was used.

## Finding RUST-1 — BLOCKER: Rust constants do not preserve the UAPI macro and guard contract

`tty.h:2-3` defines the `_UAPI_LINUX_TTY_H` preprocessor include guard, and
`tty.h:10-44` defines every line-discipline name as an object-like C macro.
The candidate instead declares module-scoped `pub const ...: i32` items and
does not provide any C preprocessor/export bridge or guard-equivalent
mechanism. A Rust item is resolved after parsing; it cannot reproduce textual
macro expansion, the C preprocessor namespace, use in preprocessing contexts,
or the include-guard behavior required by a selected UAPI header. Assigning
all literals the Rust type `i32` also fixes their type before their C-context
integer conversion/promotion behavior can be established.

The frozen `SYMBOLS.tsv` records label both architectures' guard and all
`N_*`/`NR_LDISCS` macros `PENDING_REVIEW`; the proposal changes their
selection expressions to the generic `SOURCE_REVIEWED_VALUE` without naming a
source-proven preservation boundary. The pinned header and frozen records do
not establish an exact Rust-side representation that retains this C/UAPI macro
contract. Therefore I cannot approve closing those semantic records or the
candidate as an exact translation.

There are no pointers, references, unsafe blocks, allocation paths, drops, or
Rust layout declarations in this candidate; consequently no additional
ownership/provenance/layout finding applies.

Disposition: FINDINGS. The applier must block this task unless it can derive
an exact, frozen-source-backed UAPI macro/export and integer-context mapping;
it must not treat the `pub const i32` substitutions as parity.

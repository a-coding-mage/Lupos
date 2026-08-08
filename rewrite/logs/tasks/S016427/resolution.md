# Resolution — S016427 / include/uapi/linux/tty.h

## Disposition: BLOCKED

The sealed candidate is not changed.  The pinned source establishes an
interface whose required C-facing preservation boundary is absent from the
frozen records and candidate; manufacturing one in this single Rust file would
both exceed its frozen scope and be an unreviewed design.

### P001 — accepted; unresolved include-guard interface

`vendor/linux/include/uapi/linux/tty.h:2-3,46` defines and tests
`_UAPI_LINUX_TTY_H` in the C preprocessor.  The candidate has no C header,
preprocessor adapter, or ABI record that could make its Rust module establish
that macro or respond to repeated C inclusion.  The corresponding selected
`SYMBOLS.tsv` records for both architectures remain `PENDING_REVIEW`; the
proposal's generic `SOURCE_REVIEWED_VALUE` is not a source-derived guard
mapping.  The review finding is accepted.

### P002 — accepted; unresolved macro namespace and integer-context contract

`vendor/linux/include/uapi/linux/tty.h:10-44` exports `N_TTY` through
`N_CAN327` and `NR_LDISCS` as object-like C macros.  `include/linux/tty.h:12`
includes that UAPI header, and `drivers/tty/tty_ldisc.c:47,62,144,189,195`
uses the resulting tokens as an array bound, comparison operands, and
conditional-expression operands.  The candidate substitutes Rust-module
`pub const ...: i32` items, which neither expose C tokens in the unqualified
preprocessor namespace nor preserve the macros' C integer-expression context.
No pinned ABI/driver-ABI record names a retained C header or bridge that owns
this contract.  The review finding is accepted.

### RUST-1 — accepted; fixed Rust types cannot close the pending C semantics

The selected macro `selection_expression` records for both x86_64 and aarch64
are still `PENDING_REVIEW` in the frozen `SYMBOLS.tsv`.  The header's unsuffixed
integer literal macros acquire type and promotion behavior from C use context;
the candidate fixes all of them to `i32` before any such context.  Pinned
source and frozen manifests provide no exact Rust-to-C macro/export interface
or ownership boundary to resolve that difference.  The review finding is
accepted.

No compiler, formatter, linker, test, analyzer, or runtime command was used.

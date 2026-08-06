# Applier resolution — S015110

Reviewed the complete pinned header `vendor/linux/include/linux/sunrpc/xprtrdma.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`, its selected transport consumer
`net/sunrpc/xprtrdma/transport.c:70,81-82`, the frozen target commands, and both
independent review reports.

## RUST-1 — resolved

The fieldless Rust `#[repr(C)]` enum was removed.  Although its discriminants
matched the named C values, it imposed Rust nominal-enum validity and did not
preserve the C enumerators' use as ordinary `int` constant expressions.  In
particular, the pinned consumer initializes `unsigned int` values from
`RPCRDMA_FRWR` and `RPCRDMA_BOUNCEBUFFERS`, and computes `RPCRDMA_LAST - 1`.

The replacement is `pub type rpcrdma_memreg = i32` plus eight explicitly typed
integer constants from 0 through 7.  This preserves the frozen LLVM 19
commands' ordinary 32-bit signed `int` enum representation on both
`--target=x86_64-linux-gnu` and `--target=aarch64-linux-gnu` (neither command
uses `-fshort-enums`), permits the full scalar domain as C does, and supports
the selected integer initialization and arithmetic without a nominal-domain
conversion.  The two task-specific ABI rows and two lifetime rows now record
that conclusion as `COMPLETE`.

The parity reviewer reported no other finding.  No compiler, formatter,
linker, test, or runtime command was run.

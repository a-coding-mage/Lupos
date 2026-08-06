# Applier resolution — S000767

Disposition: **BLOCKED**. This is a manual source-only adjudication; no
compiler, formatter, linker, test, runtime command, or historical Rust source
was used.

## Evidence reopened

- Pinned declaration: `vendor/linux/arch/x86/include/asm/xen/trace_types.h`
  at `425f94c2954b1fe80ebdbf9b29854e89750355df`, lines 5--17.
- Frozen x86_64 consumer closure and command:
  `rewrite/SCOPE.tsv:S000767` and
  `rewrite/metadata/x86_64/compile_commands.json` (`callthunks.c`). The
  command targets `x86_64-linux-gnu` with LLVM 19, but contains neither
  `-fshort-enums` nor an emitted enum-layout record.
- Immediate upstream trace-field use:
  `vendor/linux/include/trace/events/xen.h:70--134`; operational Xen
  reference context: `vendor/linux/arch/x86/xen/multicalls.c:232--290`.
- S000767 records in `rewrite/SYMBOLS.tsv`, `rewrite/ABI.tsv`, and
  `rewrite/LIFETIMES.tsv`.

## Finding dispositions

### R1 — accepted; candidate rejected

`#[repr(C)]` fieldless Rust enums have a closed set of valid discriminants,
whereas the two ordinary C enum objects carry values in their implementation
selected compatible integer domains. The pinned trace events explicitly retain
and print fallback text for values that are not named enumerators
(`xen.h:89--101`, `118--134`). Thus the candidate cannot be retained.

The required direction is transparent integer-domain representations with all
seven named values and integer-equivalent comparison/copy behavior. Selecting
their underlying Rust integer types would, however, assert the unresolved C
ABI, so no replacement source is applied.

### R2 — accepted; blocking condition

Both enum rows in `rewrite/ABI.tsv` remain `PENDING_REVIEW` for compatible C
integer type, size, alignment, signedness, and trace-field/by-value ABI. The
available frozen command identifies the compiler and target and shows no
short-enum option, but it is not layout evidence and the materialized Phase 0
metadata has no generated type-layout or direct predicate/probe result for
these declarations. The applier cannot infer an exact integer representation
from that absence. Consequently, the ABI and related lifetime rows cannot be
truthfully closed, and this task must not reach `DONE`.

### R3 — accepted contingent on R2

C enum objects are copied and compared as compatible integers. The candidate
does not explicitly provide these operations. The eventual verified
integer-domain type must be `Copy`, `Clone`, `PartialEq`, and `Eq` and preserve
unknown bit patterns. It is not applied because R2 leaves its concrete ABI
unknown.

### Callback FFI — accepted

`xen_mc_callback_fn_t` maps the C nullable callback pointer correctly as
`Option<unsafe extern "C" fn(*mut core::ffi::c_void)>`: C ABI, raw mutable
`void *`, nullability, and no fabricated Rust reference/lifetime are retained.
The header itself transfers no ownership. The operational reference stores the
pointer and data before invoking `(*cb->fn)(cb->data)` in the flush path, so
the caller-side validity/lifetime obligation must stay explicit at the eventual
call site. This conclusion does not repair the unresolved enum ABI.

## Required unblock evidence

Materialize or regenerate, under the frozen Phase 0 identity and canonical
LLVM invocation, authoritative evidence recording the compatible C enum
representation (size, alignment, signedness and calling/trace-field ABI) for
both declarations. Reopen the Phase 0 identity/metadata gate as required by
the protocol, then resume this task and update its ABI/lifetime records before
applying a transparent, open-domain Rust representation.

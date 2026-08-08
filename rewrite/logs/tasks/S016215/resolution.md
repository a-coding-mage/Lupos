# Applier resolution — S016215 / attempt 1 / P01

## Disposition: BLOCKED

This adjudication reopened the complete pinned
`vendor/linux/include/uapi/linux/kernel-page-flags.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its direct selected inclusion
path through `vendor/linux/include/linux/kernel-page-flags.h`, and the
selected consumer `vendor/linux/fs/proc/page.c`.  It also reopened the sealed
candidate, both independent reports and their semantic-review attestations,
the task's frozen scope, symbol, file-map, ABI, and lifetime records.  No
compiler, formatter, linker, test, runtime command, analyzer diagnostic,
historical Rust source, or candidate-source edit was used.

## Finding dispositions

### R001 — selected C include guard: sustained and blocking

Pinned lines 2--3 and 40 implement C preprocessor state: the first textual
inclusion defines `_UAPILINUX_KERNEL_PAGE_FLAGS_H`, and later inclusion
suppresses all of this header's tokens.  The selected direct path confirms
that the UAPI header is textually included by
`include/linux/kernel-page-flags.h:5`, which is in turn textually included by
`fs/proc/page.c`.  The sealed candidate contains Rust items only; it does not
and cannot, within this file's frozen mapping, define the selected C macro or
establish a source-proven equivalent for repeated C textual inclusion.

The frozen task mapping names only
`src/include/uapi/linux/kernel-page-flags.rs`; it supplies no module/import
contract for the translated `include/linux/kernel-page-flags.h` or
`fs/proc/page.c` consumers.  `SYMBOLS.tsv` retains the guard conditional and
macro as selected `PENDING_REVIEW` records for both architectures.  Neither
`ABI.tsv` nor `LIFETIMES.tsv` contains an S016215 bridge decision.  Treating a
future Rust module boundary as equivalent would therefore be an unsupported
design decision, not a source-evidenced resolution.

### R002 — macro token namespace, contextual expression, and UAPI surface:
### sustained and blocking

Pinned lines 9--38 define unscoped C preprocessor tokens.  The direct selected
consumer uses those tokens as ordinary C expressions, including `1 <<
KPF_PGTABLE` and `kpf_copy_bit(..., KPF_LOCKED, ...)` in
`fs/proc/page.c:182--226`; the public UAPI header also remains a C-facing
header.  The candidate's `pub const i32` values preserve the literal values
but impose Rust item namespace, visibility, and typed-item semantics.  No
frozen record identifies the corresponding Rust import paths, contextual
conversion rules, or the C/UAPI exposure boundary for either selected target.

The missing bridge is material: substituting module imports, exported Rust
macros, raw C declarations, or a different FFI surface would each introduce
new source and an unreviewed interface design.  The pinned source and frozen
records do not choose among those alternatives.  The numeric values alone do
not prove that any one preserves the selected textual-macro and UAPI contract.

## Terminal result

No candidate change was applied because the candidate is sealed and a
source-proven correction is unavailable.  The required next evidence is a
frozen, target-bound mapping for the C textual include/guard and the Rust/C
UAPI consumer boundary.  Until that exists, no S016215 semantic record can be
closed and the task must be `BLOCKED`, not marked `DONE`.

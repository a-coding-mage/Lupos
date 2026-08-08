# S015088 applier resolution — attempt 2

Pinned source re-opened: `vendor/linux/include/linux/sunrpc/gss_err.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## P001 — BLOCKED

Disposition: **BLOCKED — source-level macro operand and result-type parity is
not established.**

The seven function-like Linux macros at lines 92-99 and 154-159 expand their
single `x` operand against masks explicitly cast to `OM_uint32` at lines 83-85.
The resulting C expression therefore uses the caller expression's integer type
and the C usual arithmetic conversions; it is not declared to accept only an
`OM_uint32` operand. The current Rust macros instead put `u32` operands
directly in `&` and `>>` expressions, which restricts the macro call to the
Rust `u32` operation domain and fixes that result domain.

The pinned-tree search found the seven definitions but no direct invocation
from which to prove a narrower selected caller contract. The frozen scope row
selects this header through `net/sunrpc/auth_gss/auth_gss.o` for both
architectures, but provides no operand-type restriction. Consequently, absence
of a current direct invocation cannot establish that the header's macro
interface may be narrowed.

A source-faithful general replacement would need to preserve C integer
promotions and usual arithmetic conversions, including their caller-dependent
result type, while retaining macro lexical behavior. The pinned header and the
frozen task records do not provide a justified Rust mapping for that public
macro contract. Adding a trait/overload layer to guess the supported expression
types would be a new, unreviewed design rather than a proven translation.

No source correction was applied. The semantic proposal's closure records for
these macros remain unresolved; no semantic final/disposition commit was made,
because a commit requires every affected record to become `COMPLETE` or
`NOT_APPLICABLE` and that would contradict this finding.

Source evidence: `vendor/linux/include/linux/sunrpc/gss_err.h:83-85,92-99,154-159`;
`rewrite/SCOPE.tsv` row `S015088`; `rewrite/logs/tasks/S015088/parity-review.md`
finding `P001`.

No compiler, formatter, linker, test, runtime, diagnostic, or Git action was
used.

# S000686 parity review (slot 1)

## Scope and evidence

- Pinned source: `vendor/linux/arch/x86/include/asm/shared/tdx_errno.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/arch/x86/include/asm/shared/tdx_errno.rs`.
- Frozen scope record: `S000686`, `x86_64`, unconditional header-closure
  selection (202 consumers).  `CONFIG_INTEL_TDX_GUEST` is disabled, but the
  source header has no Kconfig conditional, so an unconditional Rust mapping
  is required and present.

## Exhaustive mapping result

All 24 operative macro names are present exactly once in the candidate.  The
status mask and all 19 `ULL` SEAMCALL status-code macros have unchanged bit
patterns and are correctly represented as `u64` for this x86_64 task.  The
source has no functions, types, statics, control flow, storage, ABI linkage,
locking, allocation, or conditional branches beyond its include guard; the
Rust module supplies the equivalent one-definition property.  Candidate
provenance has the exact task, source path, Linux revision, and `x86_64`
architecture, and no branding deviation or test/placeholder was found.

## Finding

### P1 — Operand-ID constant types do not preserve their C literal types

Linux symbols: `TDX_OPERAND_ID_RCX`, `TDX_OPERAND_ID_TDR`,
`TDX_OPERAND_ID_SEPT`, `TDX_OPERAND_ID_TD_EPOCH`.

Evidence: lines 35–38 of the pinned header define `0x01`, `0x80`, `0x92`, and
`0xa9` without a suffix.  Under the frozen x86_64 C ABI, each is an `int`
literal (all fit in signed `int`); it is not an unsigned 32-bit literal.
Candidate lines 32–35 instead declare each as `u32`.

Impact: although the four values are identical, `u32` changes signedness and
the Rust type exposed to every use.  It does not model C's `int` type or its
usual arithmetic conversion when combined with other operands.  The applier
must use the Rust representation selected for C `int` by the frozen ABI (for
example `i32`) and ensure subsequent translated use sites perform the
corresponding explicit C-promotion/conversion semantics where required.

## Disposition

Reject pending resolution of P1.  No other parity discrepancy found.

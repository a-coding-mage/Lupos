# Applier resolution — S000713

## Inputs reopened

- Branch: `feat/bun-like-rewrite-test`.
- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`, matching
  `vendor/linux.SHA` and the immutable provenance in the candidate.
- Complete source: `vendor/linux/arch/x86/include/asm/syscalls.h` (SHA-256
  `8655e80fcb5895893bee11f1c6cd3ce58117d4a9bc650e2a9777faa8f1567b31`).
- Candidate: `src/arch/x86/include/asm/syscalls.rs` (SHA-256
  `ce98f7fac35fb25f39aeb41ed90ea0a53fcfef73d57966fd52d6b0c43e8eeaa1`).
- Both independent reports: no findings. I independently reopened the source,
  the two `ksys_ioperm` definition branches and caller in
  `arch/x86/kernel/ioport.c`, the frozen x86_64 configuration, Phase 0
  identity, S000713 scope/symbol records, header-closure evidence, and the
  ABI record for the defining S000919 task.

## Resolution

No source change is required. The complete pinned header has exactly one
operative declaration:

`long ksys_ioperm(unsigned long from, unsigned long num, int turn_on);`

The candidate declares exactly that item once as
`unsafe extern "C" { pub fn ksys_ioperm(c_ulong, c_ulong, c_int) -> c_long; }`.
An extern-C item uses the unmangled C symbol name `ksys_ioperm`; there is no
alternate calling-convention, `static`, visibility, attribute, variadic
parameter, pointer, aggregate, callback, or layout-bearing payload in the
upstream declaration. `unsafe` correctly preserves that calling the kernel
function has no fabricated safe Rust contract.

The frozen target is `x86_64-linux-gnu`, with `CONFIG_64BIT=y`,
`CONFIG_X86_64=y`, and `CONFIG_X86_IOPL_IOPERM=y`; its selected UAPI header
sets `__BITS_PER_LONG` to 64. Thus `c_ulong`, `c_long`, and `c_int` preserve,
respectively, the two unsigned-long parameters, signed-long result, and signed
`int` `turn_on` parameter under the frozen LP64 C ABI. The enabled and disabled
`CONFIG_X86_IOPL_IOPERM` branches in `ioport.c` have this same external
signature, so the declaration is configuration-invariant while the
implementation's behavior remains owned by S000919.

## Semantic-record closure

S000713's only `SYMBOLS.tsv` entries are the include guard's `ifndef`,
`define`, and `endif`: these are preprocessing idempotence machinery with no
runtime, exported-symbol, ABI, ownership, locking, RCU, refcount, or lifetime
semantics to reproduce in Rust. Their semantic disposition is closed as
not-applicable for this translation; Rust module inclusion provides no C
header-guard ABI payload. There are no rows owned by S000713 in `ABI.tsv`,
`LIFETIMES.tsv`, `DRIVER_ABI.tsv`, or `BLOCKERS.tsv`. The external C-source
ABI fact is separately and completely recorded for its implementation as
S000919 `ksys_ioperm` in `ABI.tsv`.

No branding delta, stub, test, implementation, or unreviewed redesign was
introduced. Source inspection only; no compiler, formatter, rust-analyzer,
build, test, debugger, or runtime tool was used.

## Disposition

Accepted unchanged. Both review reports are resolved as **no finding**; task
S000713 is eligible for `DONE`.

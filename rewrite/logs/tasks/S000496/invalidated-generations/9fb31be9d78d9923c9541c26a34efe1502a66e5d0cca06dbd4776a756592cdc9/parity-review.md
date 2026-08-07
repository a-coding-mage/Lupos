# Slot-1 parity review — S000496

Reviewed only the pinned `vendor/linux/arch/x86/include/asm/cpufeatures.h`
(revision `425f94c2954b1fe80ebdbf9b29854e89750355df`), the candidate
`src/arch/x86/include/asm/cpufeatures.rs`, the frozen x86_64 configuration,
and the local pinned consumer header/callers named below. The task row is
`REVIEWING` on `P01`; the checked-out ref is
`refs/heads/feat/bun-like-rewrite-test`. No compiler, formatter, linker,
test, or diagnostic tool was invoked.

## Finding 1 — HIGH: feature and bug identifier integer semantics were changed without a frozen mapping

Linux symbols `NCAPINTS`, `NBUGINTS`, every `X86_FEATURE_*`, every
`X86_BUG_*`, and the function-like macro `X86_BUG(x)` are integer macro
expressions. The pinned definitions use unsuffixed decimal integer literals
and `*`/`+` at `cpufeatures.h:8-9`, `21-519`, and `524-577`; consequently
their source type and usual-arithmetic behavior are the C integer-expression
behavior, not pointer-sized unsigned arithmetic. `X86_BUG(x)` specifically
expands to `(NCAPINTS*32 + (x))` at line 524, with the argument parenthesized
and evaluated once in the caller's C expression context.

The candidate changes all 470 object-like identifiers to `pub const ...:
usize` (`cpufeatures.rs:10-525`) and replaces `X86_BUG(x)` with
`pub const fn X86_BUG(x: usize) -> usize` at line 527. This selects a 64-bit
unsigned, pointer-sized API and forces its operand/inference rules; it is not
the pinned C integer type or the macro's caller-context expansion. For
example, the pinned consumer `arch/x86/include/asm/cpufeature.h:76-79`
accepts feature identifiers through `set_bit` and through functions declared
with `unsigned int bit`, while `_static_cpu_has` takes `u16` at line 99.
Pinned `arch/x86/boot/cpucheck.c:130,144` also forms 32-bit C masks with
`1 << X86_FEATURE_*`. A Rust expression involving these `usize` constants
instead propagates `usize` arithmetic unless each caller supplies an explicit
conversion; signedness, operand width, shift result width, and overflow
behavior therefore differ or are unestablished.

The frozen records provide no approved resolution for that substitution:
`SYMBOLS.tsv` has 472 operative macros plus four conditionals for S000496,
all `PENDING_REVIEW`, and S000496 has no row in either `ABI.tsv` or
`LIFETIMES.tsv`. The candidate must not choose `usize` (nor the function
signature) without an exact, caller-compatible mapping for each affected
identifier category. This blocks parity acceptance until the type and
macro-to-Rust mechanism are source-justified and resolved.

## Checked source coverage

- The 470 object-like `NCAPINTS`/`NBUGINTS`/`X86_FEATURE_*`/`X86_BUG_*`
  names and their token-level numeric expressions were compared across the
  complete pinned header and candidate; no omission or changed numeric
  expression was found.
- The only selected configuration conditional is `CONFIG_X86_32` at pinned
  lines 535-541. Frozen x86_64 config sets `CONFIG_X86_64=y` and has no
  `CONFIG_X86_32`; thus `X86_BUG_ESPFIX` is inactive in the approved
  configuration. The candidate's `#[cfg(target_arch = "x86")]` at line 538
  is also false for the approved x86_64 target, so it does not alter this
  active configuration result. It is not evidence that the type/mechanism
  issue above is resolved.
- Candidate provenance names the correct Linux path, revision, architecture,
  and task. It contains no `todo!`, `unimplemented!`, panic shell, test
  configuration, unsafe/FFI/layout shell, or unauthorized Lupos branding.
  `BRANDING_ALLOWLIST.tsv` contains no allowance applicable to this header.

Result: **FINDINGS** (Finding 1).

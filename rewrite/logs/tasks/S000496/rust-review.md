# Rust source review — S000496

Reviewed independently against pinned `vendor/linux` revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`,
`arch/x86/include/asm/cpufeatures.h`, and the frozen x86_64 configuration.
No compiler, formatter, test, Git command, or historical Rust source was used.

## Findings

1. **MAJOR — the replacement of `X86_BUG(x)` changes its C type and arithmetic
   contract.** The upstream function-like macro at
   `vendor/linux/arch/x86/include/asm/cpufeatures.h:524` is
   `(NCAPINTS*32 + (x))`: the literal-only portion has C `int` semantics, and
   the usual arithmetic conversions preserve the type/signedness effects of
   its one evaluated argument. The candidate instead exports
   `pub const fn X86_BUG(x: usize) -> usize` at
   `src/arch/x86/include/asm/cpufeatures.rs:527`; it rejects signed inputs,
   widens the result to the target pointer width, and makes overflow follow
   Rust unsigned overflow behavior (including a possible checked-build panic)
   rather than the source expression's C rules. The same `usize` substitution
   is applied to `NCAPINTS`, `NBUGINTS`, every feature index, and every bug
   index (for example lines 11–12 and 529–580), without a task ABI record
   establishing that replacement. Pinned consumers demonstrate the original
   conversion boundary: `arch/x86/include/asm/cpufeature.h:51–78` passes the
   macro values through `arch_test_bit` and declares
   `setup_clear_cpu_cap(unsigned int bit)`. The applier must preserve the
   source integer contract or explicitly document and implement every required
   conversion at its Linux-equivalent boundary; it must not leave the public
   macro as a `usize` function merely for Rust indexing convenience.

2. **MAJOR — `CONFIG_X86_32` was replaced with a different, unpinned
   condition and introduces out-of-scope 32-bit behavior.** Upstream guards
   `X86_BUG_ESPFIX` with `#ifdef CONFIG_X86_32` at
   `vendor/linux/arch/x86/include/asm/cpufeatures.h:535–541`. The frozen
   x86_64 configuration sets `CONFIG_64BIT=y` and `CONFIG_X86_64=y` and has
   no `CONFIG_X86_32` selection; S000496 itself is inventory-scoped only to
   `x86_64`. The candidate instead uses `#[cfg(target_arch = "x86")]` at
   `src/arch/x86/include/asm/cpufeatures.rs:538` and retains the 32-bit item
   at line 543. A Rust target triple is not the frozen Kconfig predicate, so
   this makes availability depend on a new build-time mechanism instead of the
   recorded configuration and adds unselected architecture behavior. For this
   x86_64-only task, omit the item or use only a configuration representation
   mechanically bound to the frozen `CONFIG_X86_32` result.

## Checked without a finding

- Candidate provenance at lines 1–5 names the mapped Linux header, task,
  x86_64 architecture, and pinned revision correctly.
- All 471 upstream `NCAPINTS`, `NBUGINTS`, `X86_FEATURE_*`, and `X86_BUG*`
  definition names have a corresponding candidate public item. After excluding
  the function-like `X86_BUG`, all 470 scalar macro expressions match the
  upstream expressions after whitespace/comments are removed.
- The candidate contains no `unsafe`, raw pointers, references, FFI,
  `repr`, interior mutability, allocation, callback/RCU/refcount operation,
  test configuration, TODO/unimplemented marker, panic path, or runtime
  state. Therefore no ownership, aliasing, provenance, pinning, Send/Sync,
  Drop, ABI-layout, endian, or asynchronous-lifetime issue is present beyond
  the integer and conditional-contract findings above.

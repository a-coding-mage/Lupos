# Rust semantics review — S000188

Outcome: **reject; source changes required.** Source inspection only; no compiler,
formatter, test, or runtime tool was invoked.

## Finding R1 — critical: unit constants do not preserve C macro-definedness

`src/arch/arm64/include/asm/unistd.rs:15-26,34-35` changes each empty C
preprocessor definition into a public Rust `const NAME: ()`.  A Rust item is
name-resolvable after Rust parsing; it cannot make `#[cfg(...)]` select a Rust
item and cannot model `#ifdef NAME` / `defined(NAME)` in an includer.  Thus the
candidate has replaced the source mechanism rather than preserving it.

This is operative behavior, not documentation.  The frozen AArch64 config has
`CONFIG_COMPAT=y`, so the first group is present, and the source defines
`__ARCH_WANT_SYS_CLONE` and `__ARCH_WANT_NEW_STAT` unconditionally.  Downstream
Linux selection demonstrates the effect: `include/linux/syscalls.h:532,1059`
uses `__ARCH_WANT_COMPAT_STAT64` to expose the `stat64` syscall declarations;
`include/linux/compat.h:850,854` uses the SIGPENDING/SIGPROCMASK selectors; and
`include/uapi/asm-generic/unistd.h:209,887` uses `__ARCH_WANT_NEW_STAT` while
forming its syscall namespace.  Public unit values neither select equivalent
Rust declarations nor retain a configuration-level interface for consumers.

The applier must replace these markers with an exact frozen-configuration
selection representation that the corresponding Rust consumers actually use,
and close the affected selector records.  It must not retain unit constants as
a proxy for definedness.

## Finding R2 — critical: the generated-header inclusion was discarded

`vendor/linux/arch/arm64/include/asm/unistd.h:31` includes generated
`asm/unistd_64.h`; it does not merely obtain the final `__NR_syscalls` value.
The generated artifact is recorded as S012326 in `rewrite/SCOPE.tsv` and is
consumed by 5162 selected consumers in `rewrite/metadata/header_closure.tsv`.
Its source template establishes the complete syscall-number and alias namespace
(for example `include/uapi/asm-generic/unistd.h:209-214` and `879-910`) before
defining `__NR_syscalls` at lines 866-867.  The candidate supplies no generated
binding or equivalent namespace, and instead leaves only `NR_syscalls`.

Calling S012326 `BUILD_METADATA` does not make the values and aliases exported
by the source inclusion optional.  The applier must connect this module to an
auditable exact representation of the frozen generated header, including the
selected aliases, rather than silently omitting them.

## Finding R3 — major: `NR_syscalls` is neither an alias nor the C expression type

`src/arch/arm64/include/asm/unistd.rs:37-39` hardcodes a `usize` value.  The C
definition is `#define NR_syscalls (__NR_syscalls)` after the generated-header
inclusion.  In the frozen generated source, `__NR_syscalls` is the unsuffixed
integer literal `472` (`include/uapi/asm-generic/unistd.h:866-867`), hence the
macro expression has C `int` type on the target, not an inherent pointer-sized
type.  More importantly, the C alias follows the generated definition; the
candidate has severed that dependency.  `NR_syscalls` is used as the native
syscall-table bound through the generated macro namespace (for example
`arch/arm64/include/asm/seccomp.h:23`; `arch/arm64/kernel/sys.c:60`).

The exact Rust representation must preserve the generated alias relationship
and ensure each consumer implements the necessary C integer conversions at its
use site.  A standalone `usize = 472` is not equivalent.

## Other audited points

- The ARM-private numeric macro values fit a signed 32-bit C `int`; the
  candidate's `i32` values preserve those individual expression values.  Their
  downstream C usual-arithmetic conversions still need to be made explicit in
  each Rust consumer; no additional defect is assigned here for lines 29-32.
- `#![allow(non_upper_case_globals)]` follows only inner documentation
  attributes/comments and is a module-level lint setting, not an ABI export or
  mechanism substitution by itself.
- `pub` Rust constants produce no C-linkage symbol, but public Rust visibility
  is not a faithful substitute for C macro scope or preprocessor availability.


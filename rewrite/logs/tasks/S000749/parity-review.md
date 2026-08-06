# Parity review — S000749 (slot 1)

## Result

ACCEPT — no parity finding against the pinned selected x86_64 implementation.

## Reviewed authority and scope

- Pinned source: `vendor/linux/arch/x86/include/asm/vermagic.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df` (`vendor/linux.SHA`).
- Candidate: `src/arch/x86/include/asm/vermagic.rs`, task `S000749`.
- Current Phase 0 identity binds x86_64 to target triple `x86_64-linux-gnu` and
  frozen-config digest `a1cdb40573726de54a174da53c2eac8811dd84ab0145532784a47ec1c5efa6b4`.
- `rewrite/SCOPE.tsv` classifies this header as `RUST_TRANSLATE`, x86_64 only.
  Its authoritative header-closure row records exactly one selected consumer:
  `kernel/module/main.c:kernel/module/main.o`, built into `vmlinux.a`.
- The matching current Kbuild command record for `kernel/module/main.o` uses
  `--target=x86_64-linux-gnu` and `-m64`; it includes the generated/pinned
  Kconfig headers.  No compiler-predicate inventory row names this source,
  either vermagic macro, or its selected consumer; this header contains no
  compiler feature-test predicate.

## Conditional and macro mapping

1. The frozen x86_64 configuration sets `CONFIG_X86_64=y`.  It does not set
   `CONFIG_X86_32` or any of `CONFIG_M586TSC`, `CONFIG_M586MMX`,
   `CONFIG_MATOM`, `CONFIG_M686`, `CONFIG_MPENTIUMII`, `CONFIG_MPENTIUMIII`,
   `CONFIG_MPENTIUMM`, `CONFIG_MPENTIUM4`, `CONFIG_MK6`, `CONFIG_MK7`,
   `CONFIG_MCRUSOE`, `CONFIG_MEFFICEON`, `CONFIG_MCYRIXIII`,
   `CONFIG_MVIAC3_2`, `CONFIG_MVIAC7`, `CONFIG_MGEODEGX1`, or
   `CONFIG_MGEODE_LX`.  The processor-family choice is also guarded by
   `X86_32` in `arch/x86/Kconfig.cpu`.
2. Therefore source lines 6–44 select the `CONFIG_X86_64` arm: they
   deliberately leave `MODULE_PROC_FAMILY` undefined, and none of the x86-32
   string alternatives or the source `#error` is reachable in the frozen
   task scope.  The candidate correctly does not invent a processor-family
   value.
3. Source lines 46–50 take the non-`CONFIG_X86_32` arm, so
   `MODULE_ARCH_VERMAGIC` preprocesses to the empty string-literal token
   `""`.  The candidate's `MODULE_ARCH_VERMAGIC!()` expands to that same
   literal under the bound `target_arch = "x86_64"`; this target predicate is
   true for the frozen x86_64 target and prevents an x86-only header macro
   from appearing on a non-x86 build.
4. The source include guard has no independent selected runtime or token
   behavior after Rust's one-module source mapping; no duplicate guard state
   is required.  There are no functions, objects, layouts, linkage symbols,
   allocation paths, errors, locking, or lifetime operations in this header.

## Consumer/token check

`include/linux/vermagic.h:41–46` inserts `MODULE_ARCH_VERMAGIC` directly into
the compile-time `VERMAGIC_STRING` literal sequence, and
`kernel/module/main.c:1105` initializes its file-local `vermagic[]` from that
sequence.  The candidate retains a macro, rather than replacing it with a
runtime value or a preassembled string; its sole expansion is the exact empty
literal required by the selected C branch.  It neither adds whitespace nor
supplies the x86-32 `MODULE_PROC_FAMILY` token.  The crate-exported macro makes
that literal available to the translated consumer without creating a runtime
symbol or changing the source task's x86_64 value.

The provenance fields name the pinned source, exact revision, x86_64 scope,
and `S000749`.  The candidate contains no test configuration, placeholder,
unsafe code, exported ABI item, unauthorized branding, or unselected
architecture implementation.

## Phase-0 record closure note

`rewrite/SYMBOLS.tsv` still mechanically labels the source guard, every
conditional, and both `MODULE_ARCH_VERMAGIC` alternatives `PENDING_REVIEW`.
The final applier must close those records with the above selected/unselected
mapping before `DONE`; no source change is required for that closure.

## Non-evidentiary exposure disclosure

During this review, an overly broad search accidentally displayed snippets
from `rewrite/archive/` Phase-0 TSV material.  No historical Rust source was
opened, copied, or used.  This report and its conclusion rely only on the
current pinned `vendor/linux` source and current authoritative manifests; the
archive output is expressly non-evidentiary.

# Applier resolution — S012491

## Disposition: BLOCKED

The candidate is not accepted.  No source change is made to
`src/include/acpi/platform/acgccex.rs`.

### Upstream behavior and selected contexts

The complete pinned `vendor/linux/include/acpi/platform/acgccex.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` has no declarations, layout, or
runtime code.  Its only operative constructs are the per-C-translation-unit
include guard at lines 10--11 and, at lines 20--22, `#ifdef strchr` followed
by `#undef strchr`.

`include/acpi/platform/acenvex.h:38--40` includes this header under its
`defined(__GNUC__)` branch.  The frozen selected direct paths are
`arch/arm/xen/enlighten.c` (AArch64) and `arch/x86/kernel/acpi/boot.c`
(x86_64), each through `linux/acpi.h` and `acpi/acpi.h`.  Both consumers are
`RUST_TRANSLATE` rows (S000001 and S000807), not merely retained driver
objects.  The header closure records 2,304 AArch64 and 510 x86_64 consumers,
including both `RUST_TRANSLATE` and `LINUX_DRIVER_OBJECT` classes.

The immutable Phase 0 identity pins Clang 19 at
`/usr/lib/llvm-19/bin/clang`, and both frozen configurations set
`CONFIG_ACPI=y` (with AArch64 also setting `CONFIG_XEN=y`).  The recorded
commands contain neither `-Dstrchr` nor `-Ustrchr`; a source search finds no
Linux `#define strchr`.  These facts do not establish the frozen compiler's
predefined macro state at this inclusion point.  The Phase 0 compiler-predicate
inventory contains no probe/result for `strchr`, and no authoritative
preprocessing output records it.  Source-only Phase 1 may not invoke a
compiler to fill that gap.

### Review dispositions and pending facts

* Parity finding: **sustained.** The C guard and conditional undefinition
  mutate preprocessing state for the remainder of an individual C translation
  unit.  A Rust module cannot observe or remove that state for translated
  consumers.  The comment-only candidate therefore does not implement this
  operative selected conditional.
* Rust review: its observations are accepted only as negative constraints:
  Rust has no C textual macro namespace and a Rust `strchr` item,
  `macro_rules!`, FFI facade, or `#[cfg]` would add unauthorized behavior; no
  unsafe, layout, linkage, lifetime, ownership, or driver ABI record is
  applicable to this header.
* `SYMBOLS.tsv` rows 130411--130420: `ifndef@10`,
  `__ACGCCEX_H__`, `ifdef@20` (`strchr`), and both closing `endif`s are
  resolved as C-preprocessor, per-translation-unit constructs.  They have no
  Rust item-level representation, but their exact selected effect for the
  translated consumers cannot be shown equivalent from frozen source/config
  evidence.  They remain the task's blocking semantic facts rather than an
  implicit approval.

Original Linux driver-object consumers will retain the pinned C header in the
later driver build, but that boundary does not preserve this preprocessing
effect for S000001/S000807 and the other translated consumers.  No mapping,
driver ABI contract, or allowlisted cross-language preprocessor bridge
authorizes importing such a facade.  Exact parity therefore cannot be
established under the frozen evidence, so the required disposition is
`BLOCKED`.

No compiler, formatter, linker, test, emulator, debugger, rust-analyzer
diagnostic, or historical Lupos source was used.

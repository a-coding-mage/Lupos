# Applier resolution — S014173

I independently reopened the complete pinned source
`vendor/linux/include/linux/kernel-page-flags.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the fresh candidate, the frozen
scope/configuration and header-closure records, the prerequisite UAPI
translation, the selected `fs/proc/page.c` consumer, and both independent
source reviews.  No compiler, formatter, analyzer, linker, test, runtime, or
other diagnostic command was used.

## Review dispositions

1. **Parity review: accepted.**  The complete 22-line source consists of its
   preprocessor include guard, one unconditional inclusion of
   `<uapi/linux/kernel-page-flags.h>`, and exactly ten unconditional
   kernel-only object-like macros.  The candidate's public re-export makes
   the prerequisite `S016215` UAPI KPF names available through this kernel
   header, matching C inclusion.  Its own public constants retain every
   source name and unsuffixed signed `int` value:
   `KPF_RESERVED=32`, `KPF_MLOCKED=33`, `KPF_OWNER_2=34`, `KPF_PRIVATE=35`,
   `KPF_PRIVATE_2=36`, `KPF_OWNER_PRIVATE=37`, `KPF_ARCH=38`,
   `KPF_SOFTDIRTY=40`, `KPF_ARCH_2=41`, and `KPF_ARCH_3=42`.  The deliberate
   absence of a flag at 39 is preserved.

2. **Rust review: accepted.**  `i32` preserves the signed C `int` macro
   values on both frozen LP64 targets, including their use as the `int ubit`
   argument to `kpf_copy_bit()` in the sole selected consumer.  The re-export
   introduces no object storage, layout, linkage, allocation, ownership,
   synchronization, unsafe boundary, panic path, or FFI item.  No Rust source
   change is needed.

3. **Configuration and source-context check: confirmed.**  The header has no
   configuration-selected definition beyond its multiple-inclusion guard.  It
   is selected for both frozen architectures through `fs/proc/page.o`.
   `CONFIG_ARCH_USES_PG_ARCH_2=y` in both configurations selects the
   consumer's `KPF_ARCH_2` use; `CONFIG_ARCH_USES_PG_ARCH_3=y` is set only for
   AArch64, but `KPF_ARCH_3` is nevertheless unconditionally defined in this
   header for both configurations, exactly as translated.

4. **Provenance and non-ABI status: confirmed.**  The destination retains
   the exact upstream `GPL-2.0` SPDX identifier and required immutable
   source/revision/architecture/task provenance.  The C include guard is a
   preprocessing-only identity mechanism with no Rust public item.  This
   header declares no type, function, object, export, calling convention,
   layout, alignment, driver ABI, lifetime, locking, RCU, refcount, or
   cleanup contract; no branding allowlist entry applies.

## Task-local semantic closure

All 26 S014173 `SYMBOLS.tsv` records are closed by this adjudication: for each
frozen architecture, the opening and closing include-guard directives and the
guard macro are preprocessor-only with no Rust runtime or ABI item; the ten
remaining macro records map to the identically named public `i32` constants
listed above.  The unconditional UAPI inclusion is accounted for by the
public re-export of the completed S016215 translation.  `ABI.tsv`,
`LIFETIMES.tsv`, `DRIVER_ABI.tsv`, and `BLOCKERS.tsv` contain no S014173
record, so no task-local ABI, ownership/lifetime, driver, or blocker fact is
left unresolved.

The five required evidence artifacts now exist.  This is source-translation
pipeline completion only; the candidate has not been compiled, linked,
formatted, tested, or executed.

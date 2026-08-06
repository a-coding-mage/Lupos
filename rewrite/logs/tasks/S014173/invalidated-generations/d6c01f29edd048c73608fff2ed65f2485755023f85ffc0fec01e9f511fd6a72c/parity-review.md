# Parity review — S014173

Reviewed independently against pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`. This review used source and
frozen Phase 0 records only; no compiler, formatter, analyzer, linker, test,
or runtime command was used.

## Scope and selected context

- Queue row S014173 is `include/linux/kernel-page-flags.h` to
  `src/include/linux/kernel-page-flags.rs`, architecture `common`, with
  `S016215` as its prerequisite. The queue fingerprint verifies as
  `af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`.
- `rewrite/SCOPE.tsv` records one selected built-in consumer per frozen
  architecture: `fs/proc/page.o`. `vendor/linux/fs/proc/page.c` is the sole
  direct in-tree C include site.
- The source header has no Kconfig conditionals: it includes
  `<uapi/linux/kernel-page-flags.h>` unconditionally and unconditionally
  defines its ten kernel-only object-like macros. Both frozen configurations
  select the header; no configuration-specific candidate branch is required.

## Findings

No parity findings.

1. The candidate retains the exact immutable provenance: source path, pinned
   revision, `common` architecture set, task ID, and `GPL-2.0` SPDX identifier.
   The upstream header contains no additional copyright notice.
2. The source's unconditional UAPI inclusion is represented by a public
   re-export of `crate::include::uapi::linux::kernel_page_flags::*`. The
   prerequisite Rust UAPI translation has the matching stable KPF names and
   values `0` through `26`; this gives users of the kernel header the same
   combined name set as C preprocessing.
3. Every kernel-only source macro has an identically named candidate `i32`
   constant with the exact value: `KPF_RESERVED=32`, `KPF_MLOCKED=33`,
   `KPF_OWNER_2=34`, `KPF_PRIVATE=35`, `KPF_PRIVATE_2=36`,
   `KPF_OWNER_PRIVATE=37`, `KPF_ARCH=38`, `KPF_SOFTDIRTY=40`,
   `KPF_ARCH_2=41`, and `KPF_ARCH_3=42`. The source macros are unsuffixed
   integer literals, whose type is `int`; `i32` preserves that value category
   for their demonstrated use as the `int ubit` argument to
   `kpf_copy_bit()` in `fs/proc/page.c`.
4. The selected direct consumer uses these constants only as the second
   `int` argument to `kpf_copy_bit()`; `CONFIG_ARCH_USES_PG_ARCH_2` enables
   the `KPF_ARCH_2` use in both frozen configs, while no configuration alters
   this header's definitions. The candidate adds no ABI symbol, layout,
   storage, configuration behavior, or executable side effect.
5. The candidate accurately retains the source's non-contractual warning in
   meaning (kernel hacking assistance, subject to change, not to be relied
   upon). No branding allowlist entry applies.

## Disposition

Accepted for source parity review. No source change is requested.

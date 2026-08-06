# Applier resolution — S000803 (attempt 2)

I independently reopened only the pinned source, the current candidate, the
frozen x86_64 configuration, scope/symbol/header-closure metadata, the
relevant Kbuild/source-generation rules, and the two current review reports.
This was source-only work; no compiler, formatter, linker, test, or historical
Lupos translation was used.

## Findings and dispositions

### P1 / R3 — upstream SPDX expression

**Resolved.** `vendor/linux/arch/x86/include/uapi/asm/unistd.h:1` is exactly
`GPL-2.0 WITH Linux-syscall-note`.  The candidate now retains that expression
verbatim in Rust comment form.  This is a UAPI identifier, not an allowlisted
branding change.

### R1 — non-kernel UAPI selector and generated payloads

**Blocking.** The pinned header's lines 15–23 select
`asm/unistd_32.h`, `asm/unistd_x32.h`, or `asm/unistd_64.h` when
`__KERNEL__` is absent.  The source-generation rules establish all three
interfaces: `arch/x86/include/uapi/asm/Kbuild:2-4` declares them generated,
and `arch/x86/entry/syscalls/Makefile:27-42,65` assigns their ABI-specific
syscall-number generation.

The frozen map gives no Rust destination to any of those generated headers.
In particular, `rewrite/SCOPE.tsv` classifies
`generated/x86_64/arch/x86/include/generated/uapi/asm/unistd_32.h` as
`S012431 BUILD_METADATA` and `unistd_64.h` as `S012432 BUILD_METADATA`; the
header closure records them as generated `BUILD_METADATA`.  There is no
selected `unistd_x32.h` scope row or Rust destination, despite its declared
Kbuild generation surface.  Adding a Rust module or hand-generating its
`__NR_*` macro namespace here would exceed S000803's frozen one-file mapping
and invent an unqueued ownership mapping.  The recorded kernel consumer
command defines `__KERNEL__`, but that does not remove this UAPI header's
non-kernel conditional contract.

### R2 — `__X32_SYSCALL_BIT` promotion behavior

**Blocking.** Upstream line 13 is the untyped replacement list
`0x40000000`; the preceding comment requires the C expression
`nr & ~__X32_SYSCALL_BIT` to use the normal C integer conversions at each
use.  A Rust `pub const ...: i32` preserves the signed 32-bit standalone
bit-pattern but is a typed item, so it does not provide the source macro's
per-use conversion behavior for all syscall-number widths.

No frozen Rust-facing generated syscall-number interface, consumer-width
mapping, or macro-expansion rule records how every selected UAPI selector and
`__NR_*` use must preserve the C conversion order.  A new Rust macro or cast
policy would therefore be an unreviewed, out-of-scope design rather than a
source-backed mapping.  The candidate documentation now states this limited
property rather than claiming exact expression parity.

## Final disposition

S000803 cannot be marked `DONE`: exact UAPI selection and integer-promotion
behavior require frozen path/ownership mappings for all three generated
headers and their syscall-number macro surfaces.  The candidate SPDX error is
corrected, but the missing source-backed mapping remains a concrete Phase 0
scope/ABI blocker.

# S000182 parity review (slot 1)

Reviewed source-only on 2026-08-06 against pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Scope and frozen selection

- Queue row is `S000182`, `REVIEWING`, pipeline `P01`; it maps
  `arch/arm64/include/asm/tlbbatch.h` to
  `src/arch/arm64/include/asm/tlbbatch.rs` for `aarch64`.
- The pinned AArch64 configuration has
  `CONFIG_ARCH_WANT_BATCHED_UNMAP_TLB_FLUSH=y`.  Consequently,
  `linux/mm_types_task.h` includes this header and embeds
  `struct arch_tlbflush_unmap_batch` in `struct tlbflush_unmap_batch`.

## Exhaustive comparison

The complete upstream header is an include guard and one empty declaration:
`struct arch_tlbflush_unmap_batch { };`.  It has no fields, functions,
macros, conditionals beyond the include guard, storage, linkage, or runtime
TLB operation.  The candidate has the required immutable provenance for the
same source/revision/architecture/task and declares precisely that one empty
`#[repr(C)]` Rust type.  `Copy`/`Clone` introduce no state or operation.
Rust module loading subsumes the C include guard.

The ARM64 batched-unmap mechanism is not omitted from this file: upstream
places deferred invalidation in `arch_tlbbatch_add_pending()` and the final
barrier in `arch_tlbbatch_flush()`, both in `arch/arm64/include/asm/tlbflush.h`.
Their only use of this header's type is as an opaque, stateless batch argument;
neither operation reads or writes a batch member.  The candidate does not add
or remove any mechanism from the mapped header.

## Findings

No parity findings.  The candidate covers every selected declaration and has
no missing conditional content, TLB state, or TLB behavior for this header.

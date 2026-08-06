# S000182 implementation

Source: `vendor/linux/arch/arm64/include/asm/tlbbatch.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected AArch64 configuration enables
`CONFIG_ARCH_WANT_BATCHED_UNMAP_TLB_FLUSH`. The upstream header declares only
an empty `struct arch_tlbflush_unmap_batch`; its comment establishes that ARM64
hardware performs TLB shootdown, so this batch object does not retain a CPU
mask or any other state. The Rust destination preserves that exact empty,
embedded C-layout state with no allocation, synchronization operation, or
additional storage.

The producer and consumer operations are intentionally not reproduced here:
they are in the separately mapped `asm/tlbflush.h` task S000183. Upstream
`arch_tlbbatch_add_pending()` issues the no-sync TLBI and
`arch_tlbbatch_flush()` supplies the batch synchronization; neither operation
reads or writes this empty object.

Semantic records resolved for this task:

- ABI: empty C-layout marker, no fields, no exported linkage.
- Ownership/lifetime: embedded by value in the current task's generic batch;
  it owns and aliases no resource.
- Concurrency/ordering: no local state or barrier; ordering belongs to the
  S000183 TLB-flush operations.

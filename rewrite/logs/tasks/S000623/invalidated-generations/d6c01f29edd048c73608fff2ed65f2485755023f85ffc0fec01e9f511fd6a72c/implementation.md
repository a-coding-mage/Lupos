# Implementation — S000623

Translated `arch/x86/include/asm/orc_lookup.h` to `src/arch/x86/include/asm/orc_lookup.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected x86_64 configuration has `CONFIG_UNWINDER_ORC=y`.  The Linux linker script's `ORC_UNWIND_TABLE` allocates `.orc_lookup`, defines `orc_lookup` at its beginning and `orc_lookup_end` one `unsigned int` array-past its end, based on the `_stext`/`_etext` range and `LOOKUP_BLOCK_SIZE`.  `arch/x86/kernel/unwind_orc.c` uses those anchors for the block count, indexed reads, and initialization writes.  The Rust declarations therefore use mutable foreign `u32` symbol anchors; they deliberately do not invent a fixed Rust array length or storage.

`LOOKUP_BLOCK_ORDER` and `LOOKUP_BLOCK_SIZE` preserve the C macro values and signed-`int` source expression.  The text-bound macros are represented as hygienic Rust expression macros using `addr_of!` so their result remains the linker-defined symbol address cast to x86_64 `unsigned long` (`usize`) without creating references to linker-owned storage.  The C include guard has no Rust ABI or runtime equivalent.  The C `LINKER_SCRIPT` conditional controls the C declaration branch only; linker allocation remains owned by the original linker-script mechanism described above.

Source/context inspected: `vendor/linux/arch/x86/include/asm/orc_lookup.h`, `vendor/linux/arch/x86/kernel/unwind_orc.c`, `vendor/linux/include/asm-generic/vmlinux.lds.h`, `vendor/linux/arch/x86/kernel/vmlinux.lds.S`, and `vendor/linux/include/asm-generic/sections.h`.  No branding delta applies; the header has no functions, locking, refcounting, allocation, or cleanup paths.

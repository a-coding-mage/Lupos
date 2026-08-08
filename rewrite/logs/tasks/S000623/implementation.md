# S000623 implementation gate — BLOCKED

The complete pinned source is `vendor/linux/arch/x86/include/asm/orc_lookup.h` at
revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

This header is consumed both by the x86 ORC unwinder C translation unit and by
the x86 linker script.  Its operative contract is not representable by a
source-only Rust file from the frozen evidence: `orc_lookup` and
`orc_lookup_end` are linker-defined, unsized `unsigned int[]` symbols;
`LOOKUP_START_IP` and `LOOKUP_STOP_IP` are address-valued casts of linker
symbols `_stext` and `_etext`; and the whole declaration block is omitted when
`LINKER_SCRIPT` is defined.  The frozen ABI/lifetime records for all four
linker symbols remain `PENDING_REVIEW`, and no approved Rust linker-script
bridge or exact conditional ABI mapping is recorded.

Creating a fixed-size array, an opaque pointer, a Rust function in place of the
object-like address macros, or an unconditional declaration would change the
linkage, layout, or preprocessor-visible contract.  I therefore did not create
the destination source and did not guess.  No compiler, formatter, linker,
test, runtime, or Git mutation was used.

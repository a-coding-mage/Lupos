# Implementation — S014160

Translated `include/linux/kasan-tags.h` to `src/include/linux/kasan-tags.rs`
from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source consists solely of an include guard and four integer-like macros.
`KASAN_TAG_KERNEL`, `KASAN_TAG_INVALID`, and `KASAN_TAG_MAX` retain their
unconditional values `0xFF`, `0xFE`, and `0xFD`. The source literals are
unsuffixed C hexadecimal integer constants, whose type is `int` on both frozen
targets; the Rust constants therefore use `i32` rather than introducing a tag
wrapper or narrowing behavior.

The frozen x86_64 and AArch64 configurations both state `# CONFIG_KASAN is not
set`; neither defines `CONFIG_KASAN_HW_TAGS`. Consequently, the selected
preprocessor branch is the header's `#else`, and `KASAN_TAG_MIN` is `0x00` for
the entire approved union. The `0xF0` hardware-tag branch is not represented as
a generic Rust tag policy because it is compiled out by both frozen
configurations.

Inspected pinned context: `include/linux/kasan.h`, where the invalid tag is
documented as the software-tag shadow initializer; and
`arch/arm64/include/asm/sysreg.h`, whose MTE tag-range macros are themselves
guarded by `CONFIG_KASAN_HW_TAGS`. This task has no functions, storage, ABI
layout, linkage, ownership, locking, or cleanup behavior. No branding delta
applies.

No compiler, formatter, analyzer, test, runtime command, historical Rust
source, or non-leased source file was used or changed.

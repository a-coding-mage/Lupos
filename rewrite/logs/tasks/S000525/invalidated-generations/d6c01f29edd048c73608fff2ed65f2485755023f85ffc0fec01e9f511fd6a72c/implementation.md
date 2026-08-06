# Implementation — S000525

Translated `arch/x86/include/asm/extable_fixup_types.h` to the path-preserving
`src/arch/x86/include/asm/extable_fixup_types.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

All selected masks, shifts, macro-derived encodings, segment-register fields,
flags, and exception fixup types are present.  The exception-table encodings
use signed 32-bit values, the type of `exception_table_entry.data`; the
immediate mask retains the unsigned 32-bit type of the pinned C literal.  The
three function-like C macros retain their upstream names as `const fn`s;
`wrapping_shl` preserves the intended 32-bit assembly-field bit pattern for
signed immediates, especially `-EFAULT`.
`EFAULT` is 14 in the pinned `include/uapi/asm-generic/errno-base.h` dependency.

No build, formatting, compiler, linker, test, or runtime command was run.

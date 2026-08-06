# Implementation — S014172

Translated `include/linux/kern_levels.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/linux/kern_levels.rs`.

The C string macros are represented by immutable `&str` constants with the
same byte sequences: SOH (`0x01`) followed by the original level character.
`KERN_SOH_ASCII` preserves the C character literal's `int` expression type and
value. The integer log-level macros retain C `int` width and signedness through
`core::ffi::c_int`.

No configuration-dependent branches, storage, functions, ownership, locking,
or ABI-visible definitions occur in this header. No compilation, formatting,
or tests were run.

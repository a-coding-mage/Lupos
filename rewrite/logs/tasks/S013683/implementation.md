# Implementation — S013683

Translated `include/linux/decompress/unxz.h` to
`src/include/linux/decompress/unxz.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The Rust module represents the C include guard and exposes the `unxz` ABI
declaration. `unsigned char`, `long`, `unsigned long`, `void`, `char`, and
`int` map through `core::ffi` C-compatible types. Nullable C callback function
pointers map to `Option<unsafe extern "C" fn(...)>` so both null and callable
values retain their original representation and call contract.

No compiler, formatter, test, or historical Lupos source was used.

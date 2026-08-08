# S012620 implementation

Translated `include/crypto/dh.h` into `src/include/crypto/dh.rs` for the
frozen aarch64 selection at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

`struct dh` is `#[repr(C)]` with its three `const void *` members represented
as `*const c_void` and its three `unsigned int` members represented as
`c_uint`; it derives `Copy, Clone` because the C object is a by-value bundle of
non-owning pointers and scalar lengths.  The four header declarations remain
unsafe C ABI declarations with their original pointer mutability, `char`/`int`
and `unsigned int` widths, and exported names.

The source carries the Intel Corporation and Salvatore Benedetto notice from
the pinned header. No compiler, formatter, linker, test, or runtime tool was
used.

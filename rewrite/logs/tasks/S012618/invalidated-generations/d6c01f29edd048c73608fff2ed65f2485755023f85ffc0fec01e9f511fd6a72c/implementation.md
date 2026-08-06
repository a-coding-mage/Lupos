# S012618 implementation

Source: `vendor/linux/include/crypto/ctr.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete selected header has three object-like integer macros and no
types, declarations, inline functions, or includes.  Its C include guard has
no Rust runtime or ABI counterpart.  The destination exposes all three names
as public `core::ffi::c_int` constants, preserving each macro's C integer
value: `4`, `8`, and `16` respectively.  This matches the C macro tokens'
`int` type and keeps the values usable by the RFC 3686 counter-mode consumers.

Frozen selection evidence: x86_64 has `CONFIG_CRYPTO_CTR=y`; AArch64 has
`CONFIG_CRYPTO_CTR=m`.  Header-closure evidence identifies one x86_64 and 23
AArch64 consumers.  No ownership, layout, linkage, locking, allocation, or
lifetime contract is declared by this header.

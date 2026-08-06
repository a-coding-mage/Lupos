# S015088 implementation

Translated `include/linux/sunrpc/gss_err.h` to `src/include/linux/sunrpc/gss_err.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header is unconditional for the common x86_64/AArch64 union. `OM_uint32` remains an unsigned 32-bit status-word type. All object-like GSS service, credential, status, mask, offset, major-status, and supplementary-status definitions retain their values. The six function-like macros are `const fn` taking and returning `OM_uint32`; each evaluates the input once, as the original macros do.

There are no layouts, allocation, ownership, locking, RCU, refcounting, FFI linkage, unsafe operations, or configuration-selected branches in this header. No compilation, formatting, test, or runtime command was run.

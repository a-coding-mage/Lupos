# Implementation — S016143

Translated `include/uapi/linux/hash_info.h` to `src/include/uapi/linux/hash_info.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source contains one unconditional UAPI `enum hash_algo`.  The Rust `#[repr(C)]` enum retains its C tag and all 24 ordered enumerators, with explicit integral values `0` through `23`; `HASH_ALGO__LAST` remains the terminal count sentinel with value `23`.  There are no configuration branches, functions, storage, ownership, locking, or error paths.  The immutable provenance architecture category is `common`, matching the canonical task row for this shared x86_64/aarch64 header.

Scope and queue records identify this as the common x86_64/aarch64 RUST_TRANSLATE header task.  No branding delta applies.

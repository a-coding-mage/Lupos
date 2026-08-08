# S016252 implementation — attempt 2

Translated `include/uapi/linux/mptcp_pm.h` to `src/include/uapi/linux/mptcp_pm.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source is a UAPI constant-only header. The translation preserves both macros as Rust constants and every named C enum enumerator as an `i32` constant, including implicit succession after explicit assignments and every internal `__..._MAX` sentinel. Each public `*_MAX` macro remains expressed as its corresponding sentinel minus one.

No allocation, ownership, locking, ABI structure, callable entry point, conditional configuration branch, or runtime side effect is present in this header.

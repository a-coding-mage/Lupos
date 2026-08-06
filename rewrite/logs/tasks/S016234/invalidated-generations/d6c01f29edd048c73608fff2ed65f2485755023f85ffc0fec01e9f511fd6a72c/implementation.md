# S016234 implementation

Translated `include/uapi/linux/major.h` to `src/include/uapi/linux/major.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The destination declares all 139 device-major definitions as public `i32` constants, retaining every Linux identifier and numeric value. `HD_MAJOR` remains an alias of `IDE0_MAJOR`; `UNIX98_PTY_SLAVE_MAJOR` retains its computed parenthesized expression. The C include guard has no Rust counterpart.

The header has no configuration-selected branches, functions, layouts, linkage, ownership, allocation, synchronization, or unsafe operations.

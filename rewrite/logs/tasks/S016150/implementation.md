# S016150 implementation

Mapped `include/uapi/linux/hsr_netlink.h` to
`src/include/uapi/linux/hsr_netlink.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected aarch64 UAPI header contains two anonymous C enum declarations.
Their enumerators are represented as public `core::ffi::c_int` constants, in
source order, with the two `*_MAX` macro expansions retained as derived
constant expressions.  The anonymous enums declare no named C type or object
layout.  No selected conditional or architecture-specific branch remains
beyond the C include guard.

Reviewed local HSR consumers in `net/hsr/hsr_netlink.c`; they use these values
as Generic Netlink command and attribute integer identifiers.

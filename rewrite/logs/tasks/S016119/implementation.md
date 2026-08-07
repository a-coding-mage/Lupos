# Implementation S016119

- Source: `vendor/linux/include/uapi/linux/ethtool_netlink_generated.h`
- Revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/uapi/linux/ethtool_netlink_generated.rs`
- Architecture scope: `common`

Translated all generated UAPI definitions into global Rust `pub const` values,
preserving declaration order, implicit enum numbering, explicit values, count
and max aliases, and the two string macros. C enum constants use `i32`, matching
the Linux enum underlying type; named enum tags are represented as `i32` type
aliases, and string macros remain `&str` constants.

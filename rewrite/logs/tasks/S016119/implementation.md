# S016119 implementation

- Task: `S016119`
- Pipeline: `P01`
- Attempt: `1`
- Linux source: `include/uapi/linux/ethtool_netlink_generated.h`
- Destination: `src/include/uapi/linux/ethtool_netlink_generated.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common`
- Source class: `RUST_TRANSLATE`

The complete pinned generated UAPI header was translated. The destination preserves the SPDX/generated-header provenance, generic-netlink name/version macros, the four named enum type aliases, every anonymous and named enum constant in source order, implicit increment semantics, explicit assigned values, and `*_MAX` expressions. C enum and macro integer values are represented as `i32`; the generic-netlink name retains its NUL-terminated byte representation.

Source-grounded inventory: 651 constants and 4 named enum aliases. No conditional branches or struct layouts occur in this generated header. No tests, stubs, drivers, or shared indexes were added.

Destination SHA-256 at seal: `090e238cb21a1c45eabe87a015a46b58c9acce833755e91ed8d3e9e5a8986f6a`

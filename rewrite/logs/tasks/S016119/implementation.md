# S016119 implementation

Translated `include/uapi/linux/ethtool_netlink_generated.h` to the required
path-preserving Rust destination. The pinned generated UAPI header contains
only three object-like macros and C enum declarations.

The three string/integer macros are Rust constants. Each named C enum tag is a
transparent `u32` wrapper, retaining the tag while allowing the unsigned
netlink-value range; every enumerator remains a public `u32` constant in its
original declaration order. Anonymous C enums are represented by their global
enumerator constants. `*_MAX` values retain their source `*_CNT - 1`
expressions.

Static inventory comparison extracted 652 macro/enumerator entries from the
pinned source and 652 corresponding Rust constants with identical names and
normalized values. No conditionals beyond the C include guard are present, and
no runtime behavior, allocations, locking, drivers, or tests are involved.

Source SHA-256: `0040fd317f46a62cf14e69e08bf293b99a9162317cc31fb967930233327ee029`.
Candidate SHA-256: `a905fddfdfbc7ea8dff64b81c66162c12ca87db2dd289aa523816fea49edd0b0`.

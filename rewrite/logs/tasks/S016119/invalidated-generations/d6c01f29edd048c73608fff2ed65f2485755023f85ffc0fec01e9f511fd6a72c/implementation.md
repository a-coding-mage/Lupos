# S016119 implementation evidence

Pinned source: `include/uapi/linux/ethtool_netlink_generated.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The fresh Rust destination contains the three generated public macros, all 649 generated enumerators, and the four named enum representations. Anonymous C enums are represented by their global `i32` constants; named C enum representations are `i32` aliases. Implicit enumerators retain their preceding-value-plus-one expressions, and generated maximum values retain their count-minus-one expressions.

No configuration conditionals, structures, attributes, functions, or external type dependencies occur in the source header.

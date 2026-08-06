# Implementation — S016342

Source: `vendor/linux/include/uapi/linux/psample.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected UAPI header is included by `include/net/psample.h`, whose only
selected consumer is `net/sched/cls_api.c` for both frozen architectures.  It
contains no configuration-controlled branch.  The candidate maps all three
C enumeration domains to signed 32-bit integer constant domains: C enum
enumerators are `int` constant expressions, and the in-tree consumers use the
values as generic-netlink command and attribute IDs.  Every enumerator has its
source-order value, including each private `__*_MAX` sentinel and the public
`PSAMPLE_ATTR_MAX` expression.

The UAPI comments that prescribe netlink payload widths, byte order, flag
attributes, or nested payloads are retained next to their corresponding
constants. There are no layouts, ownership transfers, cleanup paths, locking
operations, or driver ABI entries in this header.

## Applier correction

The initial aliases and borrowed byte-slice form were rejected by both
independent reviews. The final candidate preserves each named C enum tag as a
distinct `#[repr(transparent)]` `i32` wrapper while retaining the unscoped C
enumerators as `i32` constant expressions. Each string macro is now a
by-value, NUL-terminated `[u8; N]` constant; aggregate initialization uses
the array value directly and pointer use is explicit at the translated use
site with `.as_ptr()`. This matches the frozen commands' `-funsigned-char`
element representation without turning a C literal into a borrowed Rust API.

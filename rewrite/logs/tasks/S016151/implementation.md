# Implementation — S016151

Implemented `src/include/uapi/linux/hw_breakpoint.rs` from pinned
`vendor/linux/include/uapi/linux/hw_breakpoint.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The UAPI source has two anonymous C enums and no active configuration branch.
Their enumerators are represented as public `i32` constants, matching C enum
integer values. `HW_BREAKPOINT_RW` and `HW_BREAKPOINT_INVALID` retain their
source bitwise expressions rather than substituted literals. The C include
guard has no Rust counterpart.

Relevant consumers establish that the constants feed `perf_event_attr.bp_type`
and `bp_len` as integer masks and lengths in both selected architectures. No
unsafe code, storage layout, linkage symbol, allocation, or lifetime behavior
is introduced by this header.

No compiler, formatter, test, linker, or runtime command was run.

# Implementation — S013602

Translated `include/linux/clocksource_ids.h` from the pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/linux/clocksource_ids.rs`.

The source contains one unconditional C enum. The Rust `#[repr(C)]`
`clocksource_ids` enum preserves its ordered implicit integer discriminants:
`CSID_GENERIC = 0` through `CSID_MAX = 7`. It is selected for both x86_64 and
AArch64 and has no configuration-dependent branches or lifetime behavior.

Relevant pinned consumers store this enum in clocksource/timekeeping
structures and compare its identifiers; no behavior beyond those exact values
is introduced here.

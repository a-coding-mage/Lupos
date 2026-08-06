# Implementation — S016334

Translated `include/uapi/linux/posix_acl.h` to
`src/include/uapi/linux/posix_acl.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

All source macro replacement lists are C `int` expressions for the frozen
x86_64 and AArch64 union, including the parenthesized negative literal for
`ACL_UNDEFINED_ID`. The Rust constants therefore use `i32` and preserve each
public macro name and numeric value. This UAPI header declares no types,
objects, functions, configuration-selected branches, or ownership/lifetime
state.

Read context: the complete pinned UAPI header, `include/linux/posix_acl.h`,
both frozen configurations, and the task scope, symbol, ABI, lifetime,
driver-ABI, file-map, and branding records. No build, formatter, test, or
runtime command was run.

# Implementation — S016327

Translated `include/uapi/linux/personality.h` to
`src/include/uapi/linux/personality.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The two anonymous C enums are represented by named `pub const` items. Every
enumerator literal fits C `int`; each Rust item is therefore `i32`, preserving
the enumerator expression category and the original bitwise-OR expressions.
`PER_CLEAR_ON_SETID` remains an `i32` bitwise-OR expression over its four
source operands. The header has no configuration-controlled branches for the
frozen x86_64/aarch64 union and owns no storage or lifetime-bearing objects.

Read context: the pinned header, `include/linux/personality.h`, selected x86_64
and AArch64 consumers, both frozen configurations, and the task's symbol, ABI,
lifetime, scope, file-map, and driver-ABI records. No build, formatter, test,
or runtime command was run.

# S016417 implementation

Translated `include/uapi/linux/thermal.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/uapi/linux/thermal.rs` for the frozen common architecture scope.

The candidate preserves all five enum declarations in source order with
`#[repr(C)]`, their implicit C enumerator progression, all eight direct
constants, and the three `*_MAX` expressions as `c_int` values derived from
the corresponding terminal enumerator. The source has no conditional selected
code other than the C include guard, which is represented by Rust module
inclusion rather than emitted runtime/source state.

No compiler, formatter, linker, test, runtime, or historical translation
source was used.

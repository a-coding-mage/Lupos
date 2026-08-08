# S016344 implementation

Freshly translated `include/uapi/linux/psp.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/uapi/linux/psp.rs` for the common x86_64/AArch64 scope.

The pinned source is an unconditional generated UAPI header: its include guard,
two family macros, a named version enum, six anonymous integer enums, and two
multicast-group string macros. The Rust module boundary replaces the C include
guard. `psp_version` is a `#[repr(C)]` enum with the same ordinal values. Each
anonymous C enum identifier is a same-named `core::ffi::c_int` constant, which
preserves its module-level C identifier and its exact explicit/implicit ordinal
or `MAX - 1` expression. The string macros are retained as `&str` values.

Pinned direct-context evidence: the frozen scope row selects this header via
`net/core/gro.o` for both configurations. The source has no includes,
conditional feature branches, functions, structures, storage, or lifetime/
locking behavior.

No compiler, formatter, linker, test, runtime, or historical-source command
was used.

# S016241 implementation

Translated `include/uapi/linux/membarrier.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/uapi/linux/membarrier.rs` for the common x86_64/AArch64 scope.

The selected header is unconditional apart from its C include guard.  It has no
includes, structures, functions, or configuration-dependent branches.  Both
frozen configurations select it through `kernel/sched/build_utility.o` and set
`CONFIG_MEMBARRIER=y`.

The Rust file preserves both C enum ABI categories as signed `i32` type aliases
and preserves every exported UAPI command and flag constant, their `1 << n`
integer expressions, and `MEMBARRIER_CMD_SHARED` as the compatibility alias of
`MEMBARRIER_CMD_GLOBAL`.  Rust aliases are used instead of Rust enums so the
duplicate C alias value remains representable while retaining the C enum's
integer ABI.

No compilation, formatting, tests, linker, or runtime commands were run.

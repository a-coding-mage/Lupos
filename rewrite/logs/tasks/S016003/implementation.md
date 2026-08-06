# S016003 implementation record

- Pinned source: `vendor/linux/include/uapi/asm-generic/errno.h`
- Destination: `src/include/uapi/asm-generic/errno.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (`x86_64,aarch64` queue union)
- Dependency: `S016002` (`errno-base.h`), recorded `DONE` before this implementation.

The complete 127-line pinned UAPI header was read.  Its only include is
`<asm-generic/errno-base.h>`; the destination preserves that provider relation
with `pub use super::errno_base::*`.  The source has 103 `#define` directives:
one include guard plus 102 errno/alias definitions.  All 102 operative macros
are represented as public `core::ffi::c_int` constants.  The four source aliases
remain aliases of their source operands: `EWOULDBLOCK = EAGAIN`,
`EDEADLOCK = EDEADLK`, `EFSBADCRC = EBADMSG`, and `EFSCORRUPTED = EUCLEAN`.

No conditional branches, functions, storage, ownership, synchronization,
layout, or unsafe operations occur in this header.  No compilation, formatting,
test, or runtime command was run.

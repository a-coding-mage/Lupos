# S012557 implementation

- Role: implementer
- Model / effort: gpt-5.6-terra / medium
- Oracle: `vendor/linux/include/asm-generic/mcs_spinlock.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Scope: `src/include/asm-generic/mcs_spinlock.rs` only.

The source supplies one C layout: `struct mcs_spinlock`, in declaration order,
with a self-referential node pointer followed by two C `int` fields.  The Rust
candidate uses `#[repr(C)]`, a raw self pointer, and `core::ffi::c_int` for
those fields.  It intentionally has no `Copy`, `Clone`, `Drop`, ownership
wrapper, or embedded atomics: Linux publishes and observes these ordinary
fields through the one-copy-access and acquire/release primitives in the
separate locking header and its callers.

Both frozen configurations select `CONFIG_QUEUED_SPINLOCKS=y`; their generated
`asm/mcs_spinlock.h` paths are generic Kbuild mappings to this source.  Direct
qspinlock and resilient-qspinlock contexts use `next` for queue linkage,
`locked` for handoff, and `count` for nested per-CPU node indexing.  No Rust
compiler, formatter, linker, test, or runtime command was run.

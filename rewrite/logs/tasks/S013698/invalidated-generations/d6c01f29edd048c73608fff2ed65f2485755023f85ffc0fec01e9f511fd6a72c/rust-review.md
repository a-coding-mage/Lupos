# Rust semantics review — S013698

Reviewed only `src/include/linux/device-id/auxiliary.rs` against pinned
`vendor/linux/include/linux/device-id/auxiliary.h` (Linux
`425f94c2954b1fe80ebdbf9b29854e89750355df`).  The required branch was
`feat/bun-like-rewrite-test`; queue row S013698 was `REVIEWING` on P02.  The
frozen x86_64 and aarch64 configurations both select `CONFIG_AUXILIARY_BUS=y`.
No compiler, formatter, rust-analyzer, build, test, or runtime tooling was
used.

## Findings

1. **Required — `AUXILIARY_MODULE_PREFIX` loses its C-string/FFI semantics.**
   Upstream line 10 defines a C string-literal macro:
   `#define AUXILIARY_MODULE_PREFIX "auxiliary:"`.  Such a literal supplies a
   trailing NUL and decays to a `const char *` at its use sites.  The candidate
   instead exposes `pub const AUXILIARY_MODULE_PREFIX: &str = "auxiliary:";`.
   A Rust `&str` is a UTF-8 slice/fat reference, does not contain the trailing
   NUL, and cannot be passed as the C `%s` argument that the upstream core uses
   in `drivers/base/auxiliary.c:206` without a separate conversion.  The same
   macro is also consumed by `scripts/mod/file2alias.c:1349`.  Preserve a
   NUL-terminated byte/C-string representation (with explicit pointer use at
   FFI boundaries), rather than exposing the C macro as `&str`.

2. **Required — `AUXILIARY_NAME_SIZE` changes the macro's integer type.**
   Upstream line 9 defines the untyped C integer literal `40`, whose type is
   `int` and which therefore follows C integer-promotion rules in every use.
   The candidate fixes its public Rust constant to `usize`.  That is not an
   equivalent public replacement for non-array use sites: its width and
   arithmetic/cast behavior are pointer-width unsigned rather than C `int`.
   Retain a C-`int`-width representation for the translated macro and cast only
   at the Rust array-length use required by the language.

3. **Required — the FFI data record lacks C's ordinary value-copy behavior.**
   `struct auxiliary_device_id` in upstream lines 12–15 contains only a
   `char[40]` and `kernel_ulong_t`; it is freely copied by value in C.  The
   `#[repr(C)]` candidate has compatible field layout on the frozen LP64
   targets, but it does not implement `Copy` (and therefore ordinary safe Rust
   reads from a shared/reference-backed ID record move/reject rather than make
   the bytewise value copy C permits).  Both fields are bitwise-copyable, so
   derive `Copy, Clone` for the representation.  This has no layout cost and
   prevents later translated callers from gaining an unintended ownership
   restriction.

## Checked properties

`core::ffi::c_ulong` models upstream `unsigned long` on both frozen LP64
targets, and `#[repr(C)]` places `driver_data` after the 40-byte character
array with the C ABI's natural padding/alignment.  No pointers, ownership
claims, `unsafe` blocks, panicking paths, or synchronization behavior occur in
this header itself.  The three issues above must be resolved before accepting
the task.

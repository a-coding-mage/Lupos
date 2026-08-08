# Rust source review — S012622

Reviewed only the current candidate, pinned `include/crypto/ecdh.h`, the
relevant pinned ECDH helper implementation, candidate diff, and frozen
scope/symbol/ABI/lifetime records for attempt 1 / P02. No compiler,
formatter, test, runtime, or historical-source evidence was used.

## Result: APPROVE

No Rust-semantics finding.

- `Ecdh` uses `#[repr(C)]`; on the frozen aarch64 target its pointer at offset
  zero followed by `u16` has the required C alignment and trailing padding.
  The candidate introduces neither packing nor a Rust enum/bitfield ABI.
- `*mut c_char`, `u16`, `u32`, and `c_int` respectively preserve the C
  `char *`, `unsigned short`, `unsigned int`, and `int` declarations. The
  four curve constants retain their values and are represented at the C
  `unsigned int` width.
- The three declarations retain their exact C symbol spellings and C calling
  convention. They remain raw-pointer FFI operations rather than acquiring
  Rust references, slices, allocation, bounds checks, `Drop`, panic, or
  aliasing guarantees that the C interfaces do not provide.
- The decode declaration correctly leaves `Ecdh.key` as a mutable raw pointer:
  pinned `crypto/ecdh_helper.c` assigns it to storage within the immutable
  input packet after casting. No candidate safe API strengthens that
  caller-managed, input-buffer-bounded lifetime or asserts exclusive access.
- The header/helper source specifies no locking, RCU, refcount, callback,
  pinning, or cross-CPU ownership protocol. The candidate adds none; it has no
  unsafe block, `Drop`, interior mutability, or automatic resource management
  to audit.

The sealed semantic proposal's layout, ABI alignment, ownership, lifetime, and
no-locking decisions are consistent with this manual source review.

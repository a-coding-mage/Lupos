# Rust review — S016284 (slot 2)

Reviewed the complete pinned `include/uapi/linux/netfilter/xt_LOG.h` against
`src/include/uapi/linux/netfilter/xt_LOG.rs` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, for the frozen common x86_64 and
AArch64 scope.  Immediate native use was checked in `net/netfilter/xt_LOG.c`.
This was a source-only review; no compiler, formatter, diagnostic, test, or
runtime command was run.

## Result

Accepted: no Rust-semantics, numeric-type, layout, or FFI finding.

## Layout and integer audit

- `#[repr(C)] struct xt_log_info { u8, u8, [u8; 30] }` preserves the source
  declaration order and all byte-level fields: `level` at offset 0,
  `logflags` at offset 1, and `prefix` at offset 2.  Every member has
  alignment 1, so the native struct is byte-aligned and 32 bytes on both
  frozen targets, exactly matching the `sizeof(struct xt_log_info)` target
  data used by `net/netfilter/xt_LOG.c:82,92`.
- The first two source fields are `unsigned char` and are correctly `u8`.
  The frozen x86_64 and AArch64 Kbuild command metadata includes
  `-funsigned-char`; therefore the source `char prefix[30]` has the same
  0..=255 value representation as candidate `[u8; 30]`.  The array retains
  inline, fixed-size C storage; it introduces no reference, allocation,
  UTF-8, or ownership interpretation.
- All seven object-like macros are C `int` constant expressions because their
  unsuffixed hexadecimal values fit C `int`.  Their `core::ffi::c_int`
  definitions preserve the frozen targets' signed 32-bit C-int category and
  exact values, including the non-contiguous `XT_LOG_MASK = 0x2f`.  No enum
  invalid-value constraint, narrowing conversion, shift, flag wrapper, or
  debug/release-dependent behavior was introduced.
- The header contains no selected Kconfig or architecture branch other than
  its C include guard, which correctly has no Rust analogue.  The candidate
  adds no extern item, pointer/reference API, unsafe block, aliasing or
  synchronization mechanism, `Drop`, panic path, test-only code, or mutable
  global state.

## Provenance and records

- The SPDX expression, Linux source path, exact pinned revision, `common`
  architecture membership, and task ID are exact.  UAPI names are unchanged.
- `SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` still show the Phase 0
  task-specific fields as `PENDING_REVIEW`.  For final task closure, the
  applier must record the above mechanically established ABI facts for both
  architectures: `xt_log_info` is a 32-byte, alignment-1 byte sequence with
  offsets 0/1/2, `prefix` uses unsigned-char representation under the frozen
  command flags, and these macros are signed C-int values.  This is an
  evidence-record completion requirement, not a source change request.

No source, manifest, index, or queue file was edited by this reviewer.

# Rust review — S014514

Reviewed `src/include/linux/nfs_iostat.rs` against pinned
`vendor/linux/include/linux/nfs_iostat.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the immediate consumers in
`vendor/linux/fs/nfs/iostat.h` and `vendor/linux/fs/nfs/super.c`, and the
selected x86_64/AArch64 symbol, ABI, and lifetime records.

## Result

REJECT — two issues require applier resolution.

### RUST-1 — `NFS_IOSTAT_VERS` no longer has C-string semantics (high)

The C macro at `include/linux/nfs_iostat.h:25` expands to the string literal
`"1.1"`: an array containing the terminating NUL which decays to a one-word
`const char *` when passed to a C `%s` consumer.  Its actual selected consumer
at `fs/nfs/super.c:662` passes that macro to `seq_printf(..., "statvers=%s",
NFS_IOSTAT_VERS)`.

The candidate declares `pub const NFS_IOSTAT_VERS: &str = "1.1"`.  `&str` is a
two-word Rust fat reference, contains no trailing NUL, and is neither a C
character pointer nor a C string.  A direct equivalent translation of the
selected consumer therefore cannot retain the source call/FFI contract.  Use a
representation with a static NUL-terminated byte sequence and an explicitly
controlled C-character-pointer view at the consumer boundary; do not expose a
Rust `&str` as this C macro's equivalent.

### RUST-2 — enum representation claim lacks the frozen-target ABI evidence (medium)

The candidate asserts that each unforced C enum has an `int` representation and
maps both tags to `i32`.  The selected `ABI.tsv` entries for both enum tags on
both architectures (rows for S014514) still record their layout/representation
as `PENDING_REVIEW`; no cited frozen compiler/target evidence establishes that
assertion.  C leaves the compatible integer type of an unforced enum to the
implementation.  The applier must resolve and record the exact representation
for the pinned x86_64 and AArch64 LLVM configurations before accepting an ABI
mapping.  The values 0..8 and 0..27 themselves are correct.

## Other checks

- Every byte-counter and event-counter value, including both terminal bounds,
  matches the pinned source.
- The header contains no structs, unions, pointers, atomics, ownership
  transfers, or conditional configuration declarations; no Rust unsafe block
  is present or required for the enum constants themselves.
- There is no indication that a Rust nominal enum should be introduced without
  a separate validity/FFI decision: the C domains are index arguments and C
  permits integer values beyond the named enumerators.  Any replacement must
  retain that ability and must not create invalid Rust enum values.

No source files were edited by this reviewer.

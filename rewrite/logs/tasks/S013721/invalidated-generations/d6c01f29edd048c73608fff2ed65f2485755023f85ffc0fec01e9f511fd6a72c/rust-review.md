# Rust semantic review — S013721

Reviewer: `rust_reviewer` (independent source review)  
Scope: `include/linux/device-id/mhi.h` → `src/include/linux/device-id/mhi.rs`  
Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`

## Verdict

**Reject pending applier correction.**  The `mhi_device_id` representation is
otherwise ABI-correct for the two frozen targets, but two operative macros have
been translated with different language-level semantics.  A provenance-header
correction is also required by the repository protocol.

## Findings

1. **[High] `MHI_DEVICE_MODALIAS_FMT` and
   `MHI_EP_DEVICE_MODALIAS_FMT` are macros, not addressable global objects.**
   Upstream lines 9 and 12 expand to C string-literal tokens.  In particular,
   `drivers/bus/mhi/host/init.c:1423` uses
   `"MODALIAS=" MHI_DEVICE_MODALIAS_FMT` as adjacent-literal preprocessing,
   and `scripts/mod/file2alias.c:1323,1331` consumes each expansion as a format
   literal.  Candidate lines 19--20 and 25--26 instead introduce one global
   Rust allocation per value and a raw pointer constant to that allocation.
   This changes storage/address identity and linkage semantics, cannot express
   C literal concatenation, and adds APIs (`*_PTR`) that do not exist upstream.
   Preserve the compile-time macro value in a Rust construct usable by the
   translated consumers without inventing globally addressable C-equivalent
   objects or pointer aliases.  The embedded byte sequences and terminal NUL
   lengths themselves are correct (7 and 10 bytes respectively).

2. **[Medium] `MHI_NAME_SIZE` has the wrong source-language integer type.**
   Upstream line 10 is the unsuffixed integer macro `32` (C `int`), whereas
   candidate line 22 exposes a `usize`.  Although the current struct extent is
   unchanged, this is an operative macro and changes promotions, signedness,
   and public expression type for translated consumers.  Keep the macro's
   integer semantics independently from the Rust array-length expression; do
   not make `usize` the exported semantic replacement merely for array syntax.

3. **[Medium / protocol] The required immutable provenance SPDX value is not
   present.**  The task protocol requires each translated file to begin
   `// SPDX-License-Identifier: GPL-2.0-only`; candidate line 1 says
   `GPL-2.0`.  The Linux source/revision, architecture, and task-id fields on
   lines 2--5 otherwise exactly match the pinned task.

## Confirmed ABI and lifetime facts (resolving this task's pending records)

* Both recorded compile-command families define `__KERNEL__`, target 64-bit
  little-endian Linux, and use `-funsigned-char`.  Thus upstream
  `kernel_ulong_t` is an unsigned 64-bit C `long` with size/alignment 8 on both
  targets.  `pub type kernel_ulong_t = u64` has the required object
  representation; it has no standalone linkage, storage, ownership, or
  lifetime.
* `struct mhi_device_id` is a C aggregate: `char chan[32]` at offset 0 and
  `kernel_ulong_t driver_data` at offset 32, giving size 40 and alignment 8 on
  x86_64 and AArch64.  Candidate `#[repr(C)] struct` with `[u8; 32]` then `u64`
  preserves those offsets, size, alignment, byte order, and valid bit patterns.
  The `[u8; 32]` choice is correct specifically because the frozen C commands
  make `char` unsigned; it is a fixed inline array, not a Rust string or a
  trailing/flexible array.
* The header declares neither functions nor global objects, so it introduces
  no C symbol, calling convention, or externally exported data symbol.  The
  type's storage duration, ownership, and synchronization are supplied by each
  enclosing declaration.  Source users define static const ID tables whose
  final `{}` all-zero entry terminates iteration at `id->chan[0]`; string
  initializers must retain their NUL followed by zero-fill through all 32
  channel bytes.  That initialization contract belongs to each table-owning
  consumer and is not implemented by this header.
* `driver_data` is an opaque unsigned machine word, not an owned Rust pointer.
  Selected users include both scalar values and pointer-to-integer casts (for
  example `drivers/net/mhi_net.c:388--394`).  The header needs no unsafe block
  because it neither forms nor dereferences a pointer.  Any later conversion
  between `driver_data` and a pointer must be localized in the consumer with a
  documented unsafe provenance/lifetime contract; the integer field itself
  must not be replaced by a Rust reference or owning pointer.
* The include guard is C preprocessing only; its four inventory conditional
  records are active in both frozen kernel builds and need no runtime Rust
  analogue.  No enum, packed field, union, flexible/trailing member, atomic,
  lock, RCU, refcount, or drop behavior is declared here.

No compiler, formatter, test, rust-analyzer diagnostic, or historical source
was used.

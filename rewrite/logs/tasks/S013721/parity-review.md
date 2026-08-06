# Parity review — S013721

Reviewer: parity reviewer (slot 1), P01  
Scope: `include/linux/device-id/mhi.h` → `src/include/linux/device-id/mhi.rs`  
Method: manual source inspection only; no compiler, formatter, linker, test, or
rust-analyzer diagnostic was run.

## Oracle and frozen-scope evidence

- The queue verifies on `feat/bun-like-rewrite-test`; S013721 is the unique
  `common` mapping for this header.  `S014369` (`include/linux/mhi.h`) depends
  on S013721.  The frozen aarch64 configuration enables MHI as modules;
  x86_64 does not enable it.  The header itself is nevertheless selected by
  both architecture header closures.
- The complete oracle is only lines 1--24 of
  `vendor/linux/include/linux/device-id/mhi.h`: SPDX `GPL-2.0`, the
  `__KERNEL__`-guarded `kernel_ulong_t`, three macros, and
  `struct mhi_device_id`.  It declares no function, enum, static object,
  section annotation, module ABI entry, or externally linked data.
- Both frozen Kbuild command records in `rewrite/FILE_MAP.tsv` contain
  `-funsigned-char`.  Thus the field element representation is an unsigned
  8-bit `char` for this frozen union.  Both targets are LP64: `unsigned long`
  is 8 bytes with 8-byte alignment.
- Direct consumers establish the required observable uses: `include/linux/mhi.h`
  stores `const struct mhi_device_id *`; `drivers/bus/mhi/host/init.c` and
  `drivers/bus/mhi/ep/main.c` use the two format macros in adjacent-literal
  `MODALIAS=` expressions and use `id->chan[0]` as the table terminator;
  `scripts/mod/file2alias.c` passes each macro as a printf format; the MHI
  tables in `net/qrtr/mhi.c`, `drivers/net/mhi_net.c`, and the selected MHI
  modules use string-literal initializers and a final `{}` zero entry.

## Findings

### P1 — macro literals were changed into public static objects and invented pointer APIs (reject)

Oracle lines 9 and 12 are preprocessor replacements for the token sequence
`"mhi:%s"` and `"mhi_ep:%s"`.  They define neither an object nor an address,
have no linkage, and permit C adjacent-literal concatenation such as
`"MODALIAS=" MHI_DEVICE_MODALIAS_FMT` in the host/endpoint uevent paths.

The candidate instead defines `pub static MHI_DEVICE_MODALIAS_FMT: [u8; 7]`,
`pub static MHI_EP_DEVICE_MODALIAS_FMT: [u8; 10]`, and two new public
`*_PTR` raw-pointer constants.  This changes macro expansion into header-level
storage, cannot reproduce adjacent-literal use under the macro's original
name, and introduces four source identifiers with no oracle mapping.  Its
comments incorrectly call the macro literal a static object.  The NUL bytes
inside the arrays are correct for a C string literal object at a *use site*,
but do not cure the linkage, expansion, or added-API differences.

Required resolution: represent these as a Rust mechanism that preserves every
translated call site's literal/format semantics without creating a public
header static or `*_PTR` substitute, and record the concrete mapping in the
resolution.  Do not export data merely to model a C macro.

### P2 — `MHI_NAME_SIZE` changes the macro expression type (reject)

Oracle line 10 substitutes the unsuffixed C integer constant `32` (type
`int`), whereas the candidate exposes `pub const MHI_NAME_SIZE: usize = 32`.
The existing use as an array bound happens to produce 32 bytes, but this
header's operative macro is not limited to that use; its signedness, width,
and arithmetic/conversion behavior have been changed to target-word unsigned.
Use an exact translated integer-constant representation, performing an
explicit boundary conversion only where Rust array syntax requires `usize`.

### P3 — `chan` lost the oracle's const-member contract (reject)

Oracle line 20 is `const char chan[MHI_NAME_SIZE]`.  Under the frozen command
flags its 32 element bytes are unsigned, but the member remains a C
const-qualified array: consumers may inspect it (including the sentinel
`id->chan[0]`) but may not modify an ID table's channel bytes through the
struct member.  The candidate has a public mutable `[u8; MHI_NAME_SIZE]`
field, so any holder of `&mut mhi_device_id` can overwrite `chan`.  This is a
different field contract from the static-const ID tables used by the MHI
driver/module alias machinery.

Required resolution: preserve the read-only member contract without changing
the ABI layout or granting a mutable public field as a replacement behavior.

## Items that match, subject to the above fixes

- The candidate retains the exact upstream SPDX expression; the oracle has no
  separate copyright notice to retain.
- `kernel_ulong_t = u64` is the correct ABI type for both frozen 64-bit
  `__KERNEL__` configurations.  The source conditional must remain reflected
  in the final module/import design; this candidate's unconditional alias is
  not accepted as proof for non-kernel inclusion contexts such as modpost.
- `#[repr(C)]` with a 32-byte byte array followed by `u64` gives the oracle
  field order and the required frozen-target layout: `chan` offset 0, 32-byte
  extent; `driver_data` offset 32; alignment 8; total size 40.  The candidate
  correctly has no enum, function, section annotation, or module entry.
- C string-literal table initializers have their terminating NUL and `{}`
  terminators have all-zero `chan`/`driver_data` storage.  This header does
  not itself define those tables; the final type must continue to admit their
  exact layout while retaining P3's const access contract.

## Pending-record disposition required before DONE

`SYMBOLS.tsv` still has `PENDING_REVIEW` for the include guard, `__KERNEL__`
conditional, all three macros, `kernel_ulong_t`, and `mhi_device_id` on both
architectures.  `ABI.tsv` and `LIFETIMES.tsv` retain pending records for the
two types on both architectures.  Source evidence resolves their facts as
above: no ownership transfer, allocation, locking, RCU, refcount, callback,
or lifetime behavior is introduced by this header; `driver_data` is opaque
word-sized driver-private data and `chan` is an inline, read-only 32-byte
identifier.  The applier must make the final manifest dispositions in the
required resolution workflow; this review does not edit frozen manifests.

## Verdict

Reject candidate as submitted.  P1--P3 are parity defects.  They must be
resolved from the pinned source before S013721 can be accepted; no build or
test evidence was used.

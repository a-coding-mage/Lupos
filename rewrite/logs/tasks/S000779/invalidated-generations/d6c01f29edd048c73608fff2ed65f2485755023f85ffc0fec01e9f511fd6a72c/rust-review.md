# Rust review — S000779

Reviewed `src/arch/x86/include/uapi/asm/ldt.rs` against the complete pinned
`arch/x86/include/uapi/asm/ldt.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 configuration,
the task's `SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` records, and the
immediate native consumers in `arch/x86/include/asm/desc.h`,
`arch/x86/kernel/tls.c`, and `arch/x86/kernel/ldt.c`.

Result: **changes/evidence required before acceptance**.

## Findings

1. **HIGH — the implementation-defined bit-field ABI has not been established
   for the pinned Phase 0 compiler and target.**

   `ldt.h:24-40` declares seven `unsigned int` bit-fields, including the
   x86_64-only `lm`. Their allocation unit, bit order, final size, and
   alignment are implementation-defined C ABI properties. The candidate
   chooses a trailing `u32` at Rust lines 24-25 and assumes the fields occupy
   bits 0 through 7, but its evidence calls this a “GNU C” allocation while
   the frozen identity requires the LLVM 19 x86_64 invocation. The authoritative
   `ABI.tsv` record for `struct user_desc` is still `PENDING_REVIEW`; it records
   neither the required 16-byte size / 4-byte alignment nor bit offsets and
   widths. `LIFETIMES.tsv` is likewise entirely `PENDING_REVIEW`.

   This must be resolved from the pinned compiler/target/configuration evidence
   before `DONE`: establish the three leading 32-bit offsets, the final
   allocation unit's offset and width, the seven field positions/widths, and
   the 16-byte / 4-byte native ABI. If that frozen evidence confirms the
   candidate's assumptions, `#[repr(C)] { u32, u32, u32, u32 }` is the correct
   valid-bits representation; it preserves the 24 otherwise-uninterpreted
   bits and avoids creating an invalid Rust value from user bytes. Without the
   evidence, this UAPI layout remains an unproven ABI guess.

2. **MEDIUM — the x86_64 UAPI's 32-bit-caller `lm` rule is absent from the
   Rust-facing contract and must be made explicit to consumers.**

   The source comment at `ldt.h:32-39` requires every context that receives a
   `user_desc` from a 32-bit program to behave as though `lm == 0`, even when
   the physical bit is uninitialized. The candidate's `lm()` accessor at lines
   67-70 returns the raw bit with no provenance/context boundary. Native
   `desc.h:37-41` deliberately refuses to consume `lm` when filling an LDT,
   while `tls.c:198-214` only sets it for a kernel-generated result. Future
   Rust callers therefore must not use `lm()` on compat-originated raw input;
   the task's lifetime/ABI records should state this provenance-sensitive rule
   before closure. Do not “sanitize” the stored `bits` word globally: raw
   copy-to/from-user layout and its remaining padding bits must stay intact.

## Checked successfully

- The candidate retains all five selected object-like macros with the same
  unsuffixed C `int` category (`i32`) and values: `LDT_ENTRIES` 8192,
  `LDT_ENTRY_SIZE` 8, and `MODIFY_LDT_CONTENTS_{DATA,STACK,CODE}` 0, 1, and 2.
- Subject to finding 1's frozen-ABI proof, its four `u32` members have the
  intended native offsets 0, 4, 8, and 12. `#[repr(C)]` and the fixed-width
  members avoid pointer-width, endian-conversion, packing, union, or
  bitfield-validity hazards. The raw `bits` word retains every formerly
  padding/unassigned bit; all `u32` bit patterns are valid Rust values.
- Each getter masks then shifts the original field width. Each setter masks the
  incoming `u32`, changes only its assigned bit range, and preserves all other
  bits. Shifts are fixed in `0..=7`, so no debug/release-dependent shift or
  overflow path exists. No `unsafe`, allocation, panic, `Drop`, aliasing,
  synchronization, FFI function, or test-only code is present.
- The immutable provenance is exact: SPDX remains
  `GPL-2.0 WITH Linux-syscall-note`, the Linux path and revision match
  `vendor/linux.SHA`, and the selected architecture/task are x86_64/S000779.
  The `__x86_64__` source branch is correctly represented by the included
  `LM_*` constants/accessors; no i386-only Rust branch was introduced.

No compiler, formatter, build, test, or runtime command was run. This reviewer
edited only this assigned report.

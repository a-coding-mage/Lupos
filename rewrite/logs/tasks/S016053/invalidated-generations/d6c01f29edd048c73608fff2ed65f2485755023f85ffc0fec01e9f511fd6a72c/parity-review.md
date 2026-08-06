# Parity review — S016053 (slot 1)

Reviewed `src/include/uapi/linux/arm_sdei.rs` against the complete pinned
`vendor/linux/include/uapi/linux/arm_sdei.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus its selected arm64 inclusion
contexts.  This was a source-only review; no build, formatter, test, or runtime
command was run.

## Finding P1 — function-like UAPI macros were narrowed to fixed Rust function signatures (must resolve)

Linux lines 8 and 35--37 define `SDEI_1_0_FN(n)` and the three
`SDEI_VERSION_*(x)` names as function-like C macros.  Expansion retains the
operand expression and its C integer type/conversion rules: in particular,
`SDEI_1_0_FN` performs the usual arithmetic conversions with its `unsigned
int` base, while each version macro's shift and mask are evaluated in the
operand's resulting integer type.  The candidate replaces these four public
macros with `pub const fn` items constrained respectively to `u32 -> u32` and
`u64 -> u64` (lines 15 and 45--57).  That rejects valid operand forms and
changes expression type/conversion behavior exposed by this UAPI header.

The pinned in-tree SDEI driver currently passes a `u64 ver` to the three
version macros (`drivers/firmware/arm_sdei.c:958--978`) and uses only the
concrete function-number constants, so those present call sites compute the
expected numeric values.  This does not restore the macro contract for other
selected/external UAPI consumers.  Preserve the source macro semantics (or
record the exact, evidence-backed restriction if the Rust interface cannot
express them) before accepting the task.  The directly exposed mask and shift
constants should also retain their source literal-width/signedness semantics
rather than being widened solely to support the narrowed `u64` functions.

## Verified parity

- Provenance is exact: SPDX `GPL-2.0 WITH Linux-syscall-note`, source path,
  pinned revision, aarch64 scope, task ID, and the upstream copyright notice
  are present.
- Apart from the C include guard (which has no Rust-module analogue), every
  object-like UAPI macro is present with its source spelling and numeric value:
  the two function-number base/mask constants; all 17 concrete SDEI v1.0
  function IDs; the six version shift/mask constants; six firmware return
  values; two registration modes; three status bit positions; two completion
  statuses; five GET_INFO selectors; two event types; and two priorities.
- The file defines no structs, enums, typedefs, bitfields, functions with C
  linkage, or other layout/calling-convention ABI objects.  No such item is
  omitted.
- No branding delta, placeholder, test configuration, unsafe code, or source
  outside task scope was found.

Result: reject pending resolution of P1.

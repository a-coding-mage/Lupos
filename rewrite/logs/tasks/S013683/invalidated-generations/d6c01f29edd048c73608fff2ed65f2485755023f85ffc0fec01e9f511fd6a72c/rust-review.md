# Rust review — S013683

Scope: fresh, source-only review of `src/include/linux/decompress/unxz.rs`
against `vendor/linux/include/linux/decompress/unxz.h`, the frozen x86_64 and
AArch64 metadata/configuration context, and direct `unxz` call/definition
context. No prior task evidence was read and no compiler, formatter, linker,
or test command was run.

## Findings

1. **MAJOR — `error` has the wrong signed byte element type.** The candidate
   declares `error` as `Option<unsafe extern "C" fn(*mut c_char)>`. Both
   selected `lib/decompress_unxz.c` compile commands carry `-funsigned-char`,
   so the pinned C declaration's `char *` is an unsigned-byte pointer for both
   approved architectures. `core::ffi::c_char` follows the Rust target's
   normal C-char convention and does not encode this per-command override;
   its use therefore exposes a signed-byte pointer type rather than the frozen
   source ABI. Represent the callback argument as `*mut c_uchar` and remove
   the misleading `c_char` import. The pointer representation is unchanged,
   but the public raw-callback type and any value interpretation must retain
   the selected C signedness.

2. **MAJOR — the binding claims the `error` callback is nullable although the
   implementation requires it.** The candidate documentation says every
   callback is nullable and models `error` as `Option`. In the direct
   definition (`lib/decompress_unxz.c:366-394`), every allocation or decoder
   failure invokes `error(...)` without a null test. `fill` and `flush` are
   conditionally nullable as implemented, but a valid `unxz` call must supply
   `error` for all reachable failure paths. Express this callback as a
   non-null `unsafe extern "C" fn(*mut c_uchar)` and document its buffer and
   callback-lifetime obligation; retain `Option` only for `fill` and `flush`.

## Checked aspects

- `unsigned char *` input/output arguments use mutable `c_uchar` pointers;
  `long`, `unsigned long`, and `int` use the corresponding C ABI scalar types.
- `fill` and `flush` preserve the C calling convention, raw mutable `void *`
  provenance, signed `long` result, unsigned `long` size, and nullability.
- The external function declaration keeps the exact `unxz` spelling and C
  calling convention. No Rust references, slices, ownership transfer, or
  lifetime extension is introduced by the candidate.

## Disposition

Reject pending correction of both findings. The applier must recheck the
corrected raw callback types against the frozen `-funsigned-char` command
context and direct failure paths before closing the task.

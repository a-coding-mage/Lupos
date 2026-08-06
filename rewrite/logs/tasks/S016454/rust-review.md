# Rust semantics review — S016454

Reviewed `src/include/uapi/linux/vesa.rs` against pinned
`vendor/linux/include/uapi/linux/vesa.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the S016454 scope/symbol/ABI/lifetime
records, frozen x86_64 and aarch64 configuration evidence, and pinned
`include/uapi/linux/fb.h`/in-tree consumers.  This was a source-only review; no
compiler, formatter, rust-analyzer, build, test, debugger, or runtime command
was used.

## Findings

1. **Reject — `VESA_BLANK_MAX` is missing from the module-level public API.**
   Linux `enum vesa_blank_mode` at `include/uapi/linux/vesa.h:6-16` declares
   `VESA_BLANK_MAX = VESA_POWERDOWN` as an enumerator.  Like the preceding
   enumerators, its identifier is available unqualified in every C inclusion
   context; it has no separate macro merely because the enumerator itself
   already provides that identifier.  The candidate exposes it only as
   `vesa_blank_mode::VESA_BLANK_MAX` (`vesa.rs:18`) and provides module-level
   constants only through `VESA_POWERDOWN` (`vesa.rs:21-24`).  This changes the
   translated header API and prevents a direct unqualified translation of
   source such as `mode <= VESA_BLANK_MAX` in `drivers/tty/vt/vt.c:4657`.
   Provide the root-level `VESA_BLANK_MAX` with the same integer value and
   ensure the enum/value API remains consistent.

2. **Reject — enum representation and signedness are assumed rather than
   established for the frozen ABI.**  The source declares the named C enum
   `enum vesa_blank_mode`; the candidate substitutes
   `#[repr(transparent)] pub struct vesa_blank_mode(pub i32)` (`vesa.rs:8-9`).
   `repr(transparent)` guarantees the ABI of its `i32` field, not the ABI of
   the C enum.  The frozen ABI records for this exact type on both x86_64 and
   aarch64 remain `PENDING_REVIEW` for layout/alignment, and the lifetime
   records likewise remain pending.  No pinned source evidence reviewed here
   establishes that the C toolchain's enum-compatible integer type is signed
   `i32`; C enum representation is an ABI property and must not be selected by
   convenience.  This is material where the named enum crosses function/static
   interfaces (for example, `con_blank` consumers and the static enum objects
   in `drivers/tty/vt/vt.c` and `drivers/video/console/vgacon.c`).  Establish
   the exact ABI for both frozen targets and encode it explicitly; do not close
   the corresponding pending ABI/lifetime records without that evidence.

3. **Reject — provenance license identifier differs from the pinned UAPI
   source.**  The candidate declares `GPL-2.0-only` (`vesa.rs:1`), whereas the
   oracle declares `GPL-2.0 WITH Linux-syscall-note` (`vesa.h:1`).  The task is
   a UAPI header translation and the project rules require retaining the
   upstream SPDX identifier.  Restore the exact upstream identifier unless an
   explicit branding/license allowlist authorizes this difference (none was
   found for this header).

No `unsafe`, allocation, panic, `unwrap`, `expect`, test configuration, or
runtime behavior appears in this candidate.  The numeric expressions for the
four exported value constants compute 0, 1, 2, and 3 as in the oracle, but they
do not resolve the findings above.

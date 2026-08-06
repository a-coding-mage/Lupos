# Parity review — S013591

## Scope and method

Reviewed the pinned `vendor/linux/include/linux/circ_buf.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/circ_buf.rs`, the S013591 task row, and relevant pinned
Linux call sites. This report is based on manual source inspection only. No
compiler, formatter, linker, test, runtime tool, or compiler-backed diagnostic
was run or used as evidence.

## Result: findings present

1. **`CIRC_CNT_TO_END` and `CIRC_SPACE_TO_END`: the candidate changes the
   required evaluation count of `size`.**

   Linux `CIRC_CNT_TO_END` evaluates its `size` macro parameter in both
   `int end = (size) - (tail)` and `((size)-1)`
   (`vendor/linux/include/linux/circ_buf.h:26-29`).
   `CIRC_SPACE_TO_END` likewise evaluates `size` in its `end` initializer and
   its `((size)-1)` mask (`vendor/linux/include/linux/circ_buf.h:32-35`). The
   header's explicit single-access guarantee applies only to `head` and
   `tail` (`:23-25`); it does not authorize coalescing reads of `size`.

   The Rust macros bind `$size` once at
   `src/include/linux/circ_buf.rs:67` and `:81`, then reuse that local for both
   operations. This turns two source evaluations into one, changing the
   visible behavior for a side-effecting, volatile, or concurrently updated
   `size` expression. It is also a mechanism change rather than a harmless
   temporary: pinned callers rely on the analogous single-read properties for
   concurrent indices (for example, the comments at
   `vendor/linux/drivers/gpu/drm/msm/msm_rd.c:109-113` and `:147-151`). The
   replacement must retain the Linux macro's exact per-parameter evaluation
   count while still avoiding a second `head` or `tail` access.

2. **All four `CIRC_*` macros: Rust method calls do not preserve the C
   macros' ordinary integer conversions, and concrete pinned callers use
   mixed integer types.**

   The Linux definitions are untyped C expression macros
   (`vendor/linux/include/linux/circ_buf.h:16`, `:21`, `:26-35`). Their
   arithmetic therefore uses C's usual arithmetic conversions at each binary
   operator. The Rust candidate instead requires matching receiver/argument
   types for `wrapping_sub`, `wrapping_add`, and `&` (notably
   `src/include/linux/circ_buf.rs:33`, `:48`, `:68-70`, and `:82-86`), with
   `end as _` merely forcing `end` to the selected receiver type rather than
   reproducing the next C conversion.

   This is exercised by the pinned `msm_perfcntr` caller:
   `vendor/linux/drivers/gpu/drm/msm/msm_perfcntr.c:20-27` passes
   `stream->fifo.head`/`tail` to `CIRC_CNT`, `CIRC_CNT_TO_END`, and
   `CIRC_SPACE`, while `:221` passes an `int` `*head` and an `int` tail to
   `CIRC_SPACE_TO_END`; the owning structure declares `fifo_size` as `size_t`
   in `vendor/linux/drivers/gpu/drm/msm/msm_perfcntr.h:99-104`. C converts the
   `int` operands with that `size_t` operand at each operation. The candidate
   instead attempts, for example, `size_t::wrapping_sub(int)` and then a
   bitwise `&` between values constrained to incompatible operand types; it
   supplies neither the C conversion nor an equivalent per-call typed mapping.
   The candidate's comment at `circ_buf.rs:58-61` asserts equivalence for an
   `unsigned long` head but does not cover this concrete mixed-type caller.
   A faithful translation must preserve the frozen call-site widths, signs,
   promotions, conversion points, and final C `int` temporaries rather than
   exposing one homogeneous Rust-integer macro interface.

## Items with no additional finding

`struct circ_buf` retains the source field order and `int`-width index fields;
the frozen command evidence includes `-funsigned-char`, so the candidate's
`*mut u8` buffer element is consistent with the pinned C character mode. The
header contains no configuration branches, functions, linkage definitions, or
allowlisted branding deltas beyond the four macros and this layout.

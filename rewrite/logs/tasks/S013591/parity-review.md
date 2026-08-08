# Parity review — S013591

Result: FINDINGS

## Findings

1. **P1 — CIRC_CNT_TO_END / CIRC_SPACE_TO_END: no faithful C integer-conversion contract.**
   - Pinned evidence: `include/linux/circ_buf.h:26-35` declares `int end` and `int n`; C therefore applies the usual integer promotions/conversions to each assignment and expression. `CIRC_CNT` and `CIRC_SPACE` likewise retain ordinary C arithmetic-conversion semantics.
   - Direct selected-consumer evidence: `drivers/input/serio/userio.c:41-42,115-123` passes `u8` `head`/`tail` and integer `USERIO_BUFSIZE` to `CIRC_CNT_TO_END`. The C statement expression promotes the byte operands for its `int` temporaries.
   - Candidate evidence: `src/include/linux/circ_buf.rs:15-39` calls inherent `wrapping_sub`/`wrapping_add` on the supplied expressions and leaves the temporary type inferred. Those methods require the right operand to have the receiver type, do not express C usual arithmetic conversions, and do not force the two `_TO_END` locals to C `int`. The candidate consequently has no source-proven behavior for the retained byte/int use, mixed signed/unsigned operands, or the C `int` result domain.
   - Required disposition: do not mark complete until a frozen-manifest-backed Rust representation establishes the exact operand/result domains and C conversion/overflow contract for every selected consumer; otherwise block.

2. **P1 — `_LINUX_CIRC_BUF_H` include-guard semantics omitted.**
   - Pinned evidence: `include/linux/circ_buf.h:6-7,37` uses `_LINUX_CIRC_BUF_H` to prevent duplicate declarations and macro definitions within a C translation unit; it is selected as an operative macro for both architectures in `rewrite/SYMBOLS.tsv`.
   - Candidate evidence: `src/include/linux/circ_buf.rs:1-39` provides no mapped guard or source-backed equivalent, while exporting four global macros. The frozen semantic record remains `PENDING_REVIEW` for the guard on both architectures.
   - Required disposition: establish and record the Rust module/import mechanism that is semantically equivalent for selected consumers, including duplicate import/definition behavior, or block rather than treating the guard as closed.

## Checked without finding

- `struct circ_buf` preserves field order and uses `#[repr(C)]`; the pointer-plus-two-`int` shape is represented as `*mut c_char`, `i32`, `i32`.
- The candidate preserves the macro argument evaluation counts for the ordinary, side-effect-free calls inspected: `CIRC_CNT`/`CIRC_SPACE` evaluate each supplied argument once, while each `_TO_END` macro evaluates `size` twice and `head`/`tail` once. It does not, however, establish the C operand evaluation-order and integer-conversion contract in finding 1.

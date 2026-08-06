# Application resolution — S013804

Applier: `applier`  
Model/effort: `gpt-5.6-terra` / `high`

## Source recheck

I reopened the complete pinned `include/linux/dsa/brcm.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen AArch64 header-closure
and configuration evidence, and the complete selected call contexts:

- `net/dsa/tag_brcm.c:94,130`: `dp->index` is `unsigned int` and `queue` is
  `u16`.
- `drivers/net/ethernet/broadcom/bcmsysport.c:2268,2276-2277`: `queue` is
  `u16`; both extraction results assign to `unsigned int`.
- `include/net/dsa.h:261`: `struct dsa_port::index` is `unsigned int`.

The AArch64 frozen configuration selects this header through
`net/dsa/tag_brcm.o`; header-closure evidence also records the retained
original driver-object consumer `drivers/net/ethernet/broadcom/bcmsysport.o`.
The complete header has only its include guard and the three unconditional
macros. It has no declarations, data, layouts, linkage, ownership, locking,
allocation, error, or configuration-controlled runtime behavior.

## Findings and dispositions

### R1 — unparenthesized `q` replacement-list precedence

**Resolved.** The source definition is exactly `((p) << 8 | q)`: `p` is
parenthesized and `q` is intentionally not. `BRCM_TAG_SET_PORT_QUEUE` now
captures the right argument as tokens and transcribes `$($q)+` directly as the
right operand of `|`; it does not surround it with Rust parentheses or a cast.
Thus the supplied right-hand token sequence retains its normal operator
precedence and is evaluated once, just as it is in the source replacement
list. For the only selected invocation, the bare `u16 queue` value is accepted
by the local `BitOr<u16>` implementation, which performs the C unsigned-int
conversion at the `|` operation rather than changing the right argument's
expression binding.

### R2 — 32-bit unsigned left-shift overflow behavior

**Resolved.** On the frozen AArch64 target, `unsigned int` is the selected
32-bit operand type. The source shifts that unsigned value left by the
constant 8, whose count is within its width; C reduces the resulting unsigned
value modulo 2^32. The candidate now casts `p` to `u32` and uses
`.wrapping_shl(8)`, which expresses that total 32-bit modular operation without
a profile-dependent Rust overflow panic. `p` is evaluated exactly once.

## Final parity determination

`BRCM_TAG_GET_PORT` and `BRCM_TAG_GET_QUEUE` remain one-evaluation expression
macros with their source shift and mask operations. The candidate retains the
required immutable provenance, SPDX identifier, and Broadcom copyright notice;
it introduces no ABI symbol, layout, unsafe operation, branding delta,
placeholder, test configuration, or driver rewrite.

The Phase 0 `PENDING_REVIEW` records for this task are closed as follows:

| Record | Final source determination |
| --- | --- |
| include guard (`ifndef@8`, `_NET_DSA_BRCM_H`, `endif@16`) | Preprocessor-only single-inclusion mechanism; no Rust runtime, layout, linkage, or lifetime analogue is required. |
| `BRCM_TAG_SET_PORT_QUEUE` | Selected `unsigned int`/`u16` inputs, preserved replacement-list precedence, one evaluation of each input, 32-bit modular shift, and unsigned OR result. |
| `BRCM_TAG_GET_PORT` | Selected `u16` input is promoted non-negatively before a right shift; result matches assignment to `unsigned int`. |
| `BRCM_TAG_GET_QUEUE` | Selected `u16` input is promoted non-negatively before the `0xff` mask; result matches assignment to `unsigned int`. |
| ABI/lifetime/locking/refcount/RCU records | Not applicable: this header declares only preprocessor macros and no objects, functions, pointers, layouts, synchronization, ownership, or asynchronous state. |

No compiler, formatter, rust-analyzer diagnostic, build, link, test, runtime,
debugger, benchmark, or historical Lupos source was used.

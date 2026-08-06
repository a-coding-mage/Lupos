# Implementation — S013804

Translated `include/linux/dsa/brcm.h` to `src/include/linux/dsa/brcm.rs` from
the complete pinned Linux source at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The frozen AArch64 configuration selects this header through the module target
`net/dsa/tag_brcm.o` (`CONFIG_NET_DSA_TAG_BRCM_COMMON=m` and related enabled
Broadcom tag variants).  Header-closure metadata records two consumers: the
selected Rust translation unit `net/dsa/tag_brcm.c` and the retained original
Linux driver object `drivers/net/ethernet/broadcom/bcmsysport.o`.

The source has only an include guard and three unconditional expression macros:
`BRCM_TAG_SET_PORT_QUEUE`, `BRCM_TAG_GET_PORT`, and
`BRCM_TAG_GET_QUEUE`.  The Rust translation keeps them as exported expression
macros with the same shift, bitwise-or, right-shift, and low-octet mask
operations.  The selected producer has an `unsigned int` port index and `u16`
queue mapping, which C converts to `unsigned int`; the selected consumer
promotes its `u16` mapping to non-negative `int` and assigns the extracted
values to `unsigned int`.  The macros therefore use `u32` at those frozen
boundaries, retaining the selected C values and preventing Rust's absence of
integer promotions from changing the expression.  There are no declarations,
layouts, linkage, allocation,
ownership, locking, error paths, or configuration-controlled branches in this
header.  The destination's provenance is intentionally AArch64-only, matching
the queue row.

No branding delta applies.  No compiler, formatter, test, runtime command,
historical Rust source, or non-leased source file was used or changed.

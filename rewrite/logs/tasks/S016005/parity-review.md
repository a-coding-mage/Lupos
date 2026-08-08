# Parity review — S016005

Reviewer: parity_reviewer (gpt-5.6-terra, high)

Result: FINDINGS

## P1 — selected UAPI macro surface is replaced by non-equivalent Rust items

Linux symbols: `HUGETLB_FLAG_ENCODE_SHIFT`, `HUGETLB_FLAG_ENCODE_MASK`, and
`HUGETLB_FLAG_ENCODE_{16KB,64KB,512KB,1MB,2MB,8MB,16MB,32MB,256MB,512MB,1GB,2GB,16GB}`.

Pinned evidence: `include/uapi/asm-generic/hugetlb_encode.h:20-35` defines each
selected symbol as a C preprocessor macro.  The `14U` through `34U` left
operands intentionally give the encoded expressions C `unsigned int` operand
semantics; the shift/mask names themselves expand as untyped C integer tokens.
The candidate replaces the entire selected macro surface with `pub const`
items, including an explicitly `i32` shift/mask surface and explicitly `u32`
encoded-value surface.  A Rust item neither expands in C preprocessor input nor
has the context-dependent expression behavior of these C macro tokens.

The difference is operative in pinned direct UAPI consumers: `include/uapi/linux/mman.h:29-44`,
`include/uapi/linux/shm.h:56-70`, and `include/uapi/linux/memfd.h:23-37` define
their MAP/SHM/MFD public macro families by aliasing these exact macro names.
No source-proven frozen macro/UAPI-export bridge is present in the candidate or
the task-local frozen ABI records to establish that replacing them with Rust
constants preserves that public contract.  This is not a value-only header;
the macro expansion mechanism and C unsigned-expression behavior are part of
the selected interface.

Required disposition: provide a frozen, source-proven mapping which preserves
the selected UAPI macros (including alias expansion and C integer-expression
semantics) for both approved architectures, or block rather than claim parity.

## P2 — selected include-guard conditional/macro has no equivalent mapping

Linux symbol: `_ASM_GENERIC_HUGETLB_ENCODE_H_` (and the enclosing
`#ifndef`/`#endif`).

Pinned evidence: `include/uapi/asm-generic/hugetlb_encode.h:1-2,37` conditionally
defines the named guard macro; `rewrite/SYMBOLS.tsv` selects both the guard
macro and the `ifndef@1`/`endif@37` branches for x86_64 and aarch64.  The
candidate contains no mapped guard symbol or conditional mechanism.  Rust
module loading is not a source-proven substitute for the C preprocessor guard,
which controls what subsequent UAPI preprocessing observes.

Required disposition: establish and record an exact approved architecture-wide
mapping for the selected guard/preprocessor contract, or block the task.

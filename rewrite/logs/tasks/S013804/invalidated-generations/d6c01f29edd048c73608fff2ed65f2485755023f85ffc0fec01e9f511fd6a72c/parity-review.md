# Parity review — S013804

## Result

ACCEPT — no source-level parity findings.

## Verified scope and provenance

- Task queue row: `S013804`, `REVIEWING`, `src/include/linux/dsa/brcm.rs` from `include/linux/dsa/brcm.h`, architecture `aarch64`.
- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`; `vendor/linux.SHA` and `vendor/linux` HEAD agree.
- The aarch64 frozen configuration enables `CONFIG_MODULES=y`, `CONFIG_NET_DSA=m`, and `CONFIG_NET_DSA_TAG_BRCM=m`.
- Header-closure evidence selects this header for both `net/dsa/tag_brcm.o` and the original driver-object consumer `drivers/net/ethernet/broadcom/bcmsysport.o`.

## Exhaustive comparison

The complete pinned header contains only its include guard and the three selected operative macros.  The Rust file has the required immutable provenance (SPDX, exact Linux source/revision, `aarch64`, and task ID), preserves the Broadcom copyright notice, and supplies all three macros.

| Linux symbol | Pinned behavior and selected use | Candidate result |
| --- | --- | --- |
| `BRCM_TAG_SET_PORT_QUEUE(p, q)` | `tag_brcm.c:130` passes `dp->index` (`unsigned int`) and `queue` (`u16`); C integer conversion makes the OR operands `unsigned int`. | Evaluates each operand once, converts both to `u32`, shifts the port by 8, and ORs the queue: identical selected value/width. |
| `BRCM_TAG_GET_PORT(v)` | `bcmsysport.c:2277` passes non-negative `u16 queue`; the promoted C result is assigned to `unsigned int port`. | Evaluates once, performs the same logical right shift, and produces the identical value for the selected assignment. |
| `BRCM_TAG_GET_QUEUE(v)` | `bcmsysport.c:2276` passes non-negative `u16 queue`; the promoted C result is assigned to `unsigned int q`. | Evaluates once, masks with `0xff`, and produces the identical value for the selected assignment. |

The C include guard has no runtime/ABI object or exported symbol to reproduce in Rust.  The header defines no types, data, functions, layouts, linkage, configuration branches, or assembly/driver implementation.  `#[macro_export]` is consistent with the existing translated macro-header convention and does not create a C ABI symbol.  No branding delta, placeholder, panic, test configuration, or compiler-influenced change was found.

## Evidence inspected

- `vendor/linux/include/linux/dsa/brcm.h` (complete file; lines 1–16)
- `vendor/linux/net/dsa/tag_brcm.c` (complete selected header-use context, especially line 130)
- `vendor/linux/drivers/net/ethernet/broadcom/bcmsysport.c` (the two header uses at lines 2276–2277)
- `vendor/linux/include/net/dsa.h` (`struct dsa_port::index`, line 261)
- `rewrite/SCOPE.tsv`, `rewrite/FILE_MAP.tsv`, `rewrite/SYMBOLS.tsv`, `rewrite/PHASE0_IDENTITY.tsv`, `rewrite/metadata/header_closure.tsv`, and `rewrite/configs/aarch64/frozen.config`

No compiler, formatter, language-server diagnostics, build, test, runtime tool, or historical translation source was used.

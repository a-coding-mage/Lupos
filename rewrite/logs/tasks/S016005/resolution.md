# Resolution — S016005

Applier: `gpt-5.6-terra` / high

Result: **BLOCKED**. The sealed candidate is not edited.

## Review input and source reopened

- Pinned source reopened: `vendor/linux/include/uapi/asm-generic/hugetlb_encode.h:1-37`.
- Direct pinned UAPI consumers reopened: `vendor/linux/include/uapi/linux/mman.h:4,29-44`,
  `vendor/linux/include/uapi/linux/shm.h:6,56-70`, and
  `vendor/linux/include/uapi/linux/memfd.h:4,23-37`.
- Frozen `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, `PORTING.md`, and materialized
  metadata contain no task-specific macro/preprocessor export bridge. The selected
  conditional and operative-macro records in the frozen proposal remain source-review
  obligations rather than evidence of an alternate mechanism.

## P1 — selected UAPI macro surface

**Disposition: sustained; unresolved.**

The candidate preserves the numerical bit patterns, and the Rust review correctly notes
that every encoded literal fits the explicit `u32` expression. That fact does not close
the parity finding. The pinned header defines the selected names as preprocessor macros:
the encoded expressions retain `unsigned int` C-expression semantics through their `U`
operands. The three reopened UAPI consumer headers include this header and alias those
same macro names into the public `MAP_HUGE_*`, `SHM_HUGE_*`, and `MFD_HUGE_*` families.
A Rust `pub const` neither participates in that inclusion/alias expansion nor supplies a
C macro expression to a downstream preprocessor.

No frozen record supplies a C-compatible UAPI/macro export layer, an inter-language alias
rule, or another source-proven mechanism that preserves this selected macro contract on
both x86_64 and aarch64. Adding one would be a new unreviewed cross-file design outside
the sealed candidate and frozen task scope. Exact parity is therefore not established.

## P2 — include guard and conditional mechanism

**Disposition: sustained; unresolved.**

The pinned header's `_ASM_GENERIC_HUGETLB_ENCODE_H_` guard is selected in the frozen
symbol inventory and controls repeated textual inclusion by the C preprocessor. The
candidate has no corresponding mapping. Rust module loading is not a source-proven
replacement for the named C guard, particularly because the same header participates in
the direct UAPI includes reopened above. No frozen bridge specifies how this selected
guard macro is preserved for either architecture.

## Closure decision

The Rust review raises no independent ownership/layout issue, but it does not supply the
missing UAPI-preprocessor mapping required by the parity review. Both parity findings
remain unresolved from the pinned source and frozen records. Per the Phase 1 protocol,
this task must remain BLOCKED rather than inventing a bridge or weakening the public UAPI
contract.

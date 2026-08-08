# Resolution — S016582 / attempt 1 / P02

Applier: `gpt-5.6-terra` (high)

## Result

**BLOCKED.** The sealed candidate is not changed.  The pinned source proves
that `enum xenbus_state` is an ABI-bearing integer-storage boundary, but the
frozen AArch64 records do not establish its exact C representation.  A
source-only translation cannot safely substitute the candidate's closed Rust
enum or select a different representation by assumption.

## Finding dispositions

| Finding | Disposition |
| --- | --- |
| P1-ABI | Accepted as blocking. `include/xen/xenbus.h` embeds the type in `struct xenbus_device` and uses it in callback, parameter, and return interfaces. `rewrite/ABI.tsv` row 196765 leaves export kind, layout, alignment, and status `PENDING_REVIEW`; the pinned header supplies enumerators but no target-specific Rust ABI mapping. |
| P1-NAMESPACE | Accepted. The header declares `XenbusState*` as C ordinary-identifier enum constants; the sealed candidate exposes only Rust enum variants. No same-name compatibility mapping is present. This cannot be repaired without reopening the sealed candidate, and the ABI blocker independently prevents acceptance. |
| P1-GUARD | Accepted. Lines 10--11 of the pinned header implement `_XEN_PUBLIC_IO_XENBUS_H`; the frozen selected operative-macro record is still pending and the sealed candidate gives no source-backed compatibility mapping. |
| RUST-ENUM-FFI-001 | Accepted as blocking. `drivers/xen/xenbus/xenbus_client.c:945-959` passes `&result` to `xenbus_gather(..., "state", "%d", &result, NULL)` without checking membership in `0..8`, then returns it. `xenbus_strstate` at lines 97--111 separately returns `"INVALID"` for an out-of-range state. The source therefore requires integer storage and invalid-value behavior beyond the nine valid Rust enum discriminants. The frozen ABI evidence cannot prove a replacement's width, alignment, signedness, or FFI contract. |
| RUST-GUARD-002 | Accepted. The source-defined preprocessor gate remains an operative selected record with no completed Rust-module/compatibility mapping. |

## Blocking question

Provide frozen, target-specific evidence for the C ABI of `enum xenbus_state`
(including integer storage, width, alignment, signedness, and FFI use) and a
source-backed mapping for the selected C ordinary-identifier enumerators and
include guard. Until then, changing the candidate would be a semantic and ABI
guess, which Phase 1 forbids.

No compiler, formatter, linker, test, analyzer, historical source, or runtime
tool was used during this adjudication.

# Parity review — S016178 / P02 / attempt 1

Reviewed only `vendor/linux/include/uapi/linux/if_vlan.h`, the fresh candidate,
the candidate snapshot, and the frozen scope/symbol/ABI/lifetime records.  No
compiler, formatter, test, analyzer, or historical Lupos source was used.

## Result: FINDINGS

### P1 — C UAPI enumerator namespace and integer-constant behavior were replaced by scoped Rust enum variants

Linux `enum vlan_ioctl_cmds` (lines 21–32), `enum vlan_flags` (34–40), and
`enum vlan_name_types` (42–48) introduce the bare C identifiers
`ADD_VLAN_CMD`, `VLAN_FLAG_*`, and `VLAN_NAME_TYPE_*` as integer constants in
the including translation unit.  The candidate instead exposes only scoped
Rust variants such as `vlan_flags::VLAN_FLAG_GVRP`.  It exports no unscoped
constants with the Linux names and no integer-constant representation usable
in the ordinary C expressions preserved by selected consumers.

Pinned evidence makes this operative, rather than cosmetic: `net/8021q/vlan.c`
uses bare `VLAN_NAME_TYPE_*` in `switch` labels (lines 234–255 and 572) and
bare `VLAN_FLAG_*` values in masks, bitwise expressions, and assignments
(lines 276 and 521–565); `net/8021q/vlan_dev.c` likewise combines them with
`u32` masks (lines 221–231).  An enum variant neither preserves the C global
identifier namespace nor the C implicit integer conversion/bitmask semantics.
The frozen ABI records for all three named enums are still `PENDING_REVIEW`, so
there is no source-proven basis to make their Rust `repr(i32)` types the UAPI
contract.

Affected frozen records: the `type` entries for `enum vlan_ioctl_cmds`, `enum
vlan_flags`, and `enum vlan_name_types` for both architectures, and every
`enum_constant` entry in `SYMBOLS.tsv`.

### P1 — Frozen UAPI structure/union ABI and lifetime contract remains unclosed

Linux `struct vlan_ioctl_args` has an anonymous union at lines 54–61, whose
members are selected as `args.u.flag`, `args.u.name_type`, and so on.  The
candidate reproduces that visible nesting, but that alone is not an ABI or
lifetime resolution.

However, the external layout declaration remains unresolved in the frozen ABI
and lifetime records: `struct vlan_ioctl_args` and `anonymous_union@54` are
`PENDING_REVIEW` on both architectures.  The candidate asserts a Rust
`#[repr(C)]` equivalent but provides no source-proven resolution for the
cross-language UAPI layout, union access validity, or the C ABI/signature
contract.  This cannot be accepted as a completed translation merely from the
same apparent field ordering.

Affected frozen records: `type:struct vlan_ioctl_args` and
`type:anonymous_union@54` for x86_64 and aarch64 in `ABI.tsv` and
`LIFETIMES.tsv`.

### P1 — `char[24]` was translated as signed bytes despite the frozen selected commands

The pinned header declares `char device1[24]` and `char device2[24]` (lines
52 and 55).  The selected x86_64 and aarch64 metadata commands recorded for
this header both pass `-funsigned-char`.  The candidate uses `[i8; 24]` for
both fields.  Although the storage size is the same, reads and values at
0x80–0xff have different signed interpretation, and this UAPI buffer is
passed through `copy_from_user`/`copy_to_user` in the selected VLAN ioctl path
(`net/8021q/vlan.c` lines 509–513 and 600–612).  No frozen ABI or lifetime
record closes the resulting C character representation/byte contract.

Affected frozen records: `type:struct vlan_ioctl_args` and
`type:anonymous_union@54` for both architectures.

### P2 — Operative include guard is not mapped

`_UAPI_LINUX_IF_VLAN_H_` is an operative source guard recorded for both
architectures in `SYMBOLS.tsv` (lines 14–15 and 66).  The candidate contains
no representation or documented Rust module-level mapping for it.  A Rust
module may be loaded differently from a C header, but the frozen selected
compile-time contract remains `PENDING_REVIEW`; accepting its omission would
guess at that contract.

Affected frozen records: `ifndef@14`, `endif@66`, and
`_UAPI_LINUX_IF_VLAN_H_` for both architectures.

## Required disposition

The candidate must not advance to `DONE` on this review.  The applier must
either establish exact source-backed mappings for the unresolved UAPI enum,
layout/union, character, and guard contracts and correct the source without
changing the frozen task scope, or mark the task `BLOCKED`.

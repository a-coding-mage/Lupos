# Rust semantic review — S016178 / P02 / attempt 1 / slot 2

Reviewed only the pinned `vendor/linux/include/uapi/linux/if_vlan.h`, the
candidate snapshot, the fresh destination, the frozen SCOPE/SYMBOLS/ABI/
LIFETIMES records, and the direct pinned ioctl consumer context in
`vendor/linux/net/8021q/vlan.c`.  No compiler, formatter, analyzer, test, or
historical Lupos source was used.

## Finding R1 — unresolved C-enum ABI and value-domain substitution (BLOCKER)

`enum vlan_ioctl_cmds`, `enum vlan_flags`, and `enum vlan_name_types` are C
UAPI enum declarations.  The candidate replaces each with a Rust
`#[repr(i32)]` enum and asserts that C has an `int` representation.  Neither
the pinned header nor the frozen ABI records establishes that representation,
alignment, or exported UAPI type contract for both selected targets: each
corresponding ABI record remains `PENDING_REVIEW`.  More importantly, the
candidate replaces a C enum/type-and-integer-constant interface with Rust enum
types whose valid-value domain is only their listed discriminants; that is not
an established mapping for arbitrary integer values crossing a UAPI/FFI
boundary.  The direct ioctl payload deliberately stores `cmd` as `int`, which
does not prove the separately declared enum tags may be represented by these
Rust enum types.

The source gives the enumerator values, but it does not supply the missing
target ABI proof or an exact source-level Rust representation for the named C
enum types.  This must remain unresolved rather than be closed as COMPLETE.

Affected semantic-closure records:

- `SC1-d082235ad1e25c279881c5a95cd8508dcefd27c35a05a03aa7913eb182e3e522`
  (`aarch64`, `enum vlan_ioctl_cmds`, ABI layout)
- `SC1-a78adecb4a1ca01a4aa1bb2d5368752692fa433212e4a1844d35653ec515a4eb`
  (`aarch64`, `enum vlan_flags`, ABI layout)
- `SC1-b63da51de39de5e1d1208c42deb4ab5eb27704f91e32b7d69bf7efacbd873089`
  (`aarch64`, `enum vlan_name_types`, ABI layout)
- `SC1-5765ec5caa3d5eb529635e91fc9e625bab79a0dd8577cf5d5ec4dc50e9761e2a`
  (`x86_64`, `enum vlan_ioctl_cmds`, ABI layout)
- `SC1-223b5e15dcfe7f354fc7d17c91c4f89c3e5c7b6fc65bf30d54556e13668c16e0`
  (`x86_64`, `enum vlan_flags`, ABI layout)
- `SC1-40264a012e877368333719d2ceb92559eed00d09f51db1d0c7f20db474087320`
  (`x86_64`, `enum vlan_name_types`, ABI layout)

## Manual Rust/FFI checks

The candidate's `#[repr(C)]` union and outer struct preserve the source field
sequence, and the largest union member remains the 24-byte `char` array.  The
source itself names that union field `u`, so the candidate does not invent an
anonymous-union nesting change.  `cmd`, `VID`, the unsigned priority/name/bind/
flag alternatives, and `vlan_qos` use the corresponding fixed-width storage
types.  There are no pointers, ownership transfers, callbacks, atomics,
`unsafe` blocks, `Drop` behavior, allocation paths, or endianness conversions
in this header to audit.  These local observations cannot cure R1's missing
frozen enum ABI/value-domain decision.

## Result

FINDINGS — R1 is a source-evidence blocker.  Do not accept the candidate or
close its ABI records until the exact two-target C-enum UAPI contract and a
semantics-preserving Rust representation are established from permitted pinned
evidence.

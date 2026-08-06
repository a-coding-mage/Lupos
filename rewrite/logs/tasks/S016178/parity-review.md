# S016178 parity review (slot 1)

Reviewed `vendor/linux/include/uapi/linux/if_vlan.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/if_vlan.rs` for the frozen common header scope
(x86_64 and aarch64 consumers).

## Result

No parity findings.

## Exhaustive comparison

- Provenance is exact for task `S016178`, source path, pinned revision, and
  queue architecture class `common`; the upstream SPDX expression and
  copyright/author notice are retained. No branding delta is present or
  allowlisted.
- The three C enum-tag surfaces are represented as `c_int` aliases, and every
  unqualified enumerator is present with its C value: command values `0..9`,
  flag masks `0x1, 0x2, 0x4, 0x8, 0x10`, and name-type values `0..4`.
  The tags have no linkage or aggregate-field use in the pinned header; their
  constants retain C `int`-width values. No selected consumer passes one of
  these tag types across an ABI boundary.
- `vlan_ioctl_args_u` is a `#[repr(C)]` union with all six original members:
  `char[24] device2`, `int VID`, and the four `unsigned int` fields. The
  candidate uses `c_char`, `c_int`, and `c_uint` respectively.
- `vlan_ioctl_args` is `#[repr(C)]`, preserves the C field sequence
  `int cmd`, `char device1[24]`, union `u`, then `short vlan_qos`, and uses
  `c_int`, `[c_char; 24]`, the layout-bearing union, and `c_short`. Thus its
  natural C padding/alignment is retained (including trailing padding after
  the short).
- The pinned ioctl consumer `net/8021q/vlan.c:509-617` copies the complete
  structure and selects each union member / command by exactly these widths,
  names, and values; the candidate represents all of them. Header-closure
  evidence records this UAPI header as selected for 622 aarch64 and 195
  x86_64 consumers, with no configuration branch in the header.
- No functions, statics, macros other than the C include guard, configuration
  branches, error paths, locks, side effects, tests, placeholders, or unsafe
  code occur in the upstream header and none was omitted or introduced.

No build, formatter, test, or runtime command was run.

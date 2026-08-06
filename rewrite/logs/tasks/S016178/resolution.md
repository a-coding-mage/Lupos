# S016178 applier resolution

Task `S016178` maps pinned
`include/uapi/linux/if_vlan.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/uapi/linux/if_vlan.rs` for the frozen common (`x86_64`,
`aarch64`) scope.

## Review-report dispositions

- `parity-review.md`: accepted.  Its complete source comparison correctly
  accounts for all three enum declarations and values, the include guard, the
  `vlan_ioctl_args` field sequence, and all six anonymous-union alternatives.
  The report's conclusion is retained as source-review evidence.
- `rust-review.md`: accepted as to `repr(C)`, union shape, scalar fields,
  constants, provenance, and the absence of ownership, panic, test, or unsafe
  behavior.  During this independent applier reopen, its assertion that
  target-default `c_char` modeled the source spelling was narrowed by the
  frozen Kbuild command evidence: both recorded commands have
  `-funsigned-char`.  This does not invalidate the report's review scope, but
  it identifies a concrete ABI/API signedness defect in the submitted
  candidate.

## Applied resolution

The candidate's `device1` and `device2` fields now use `[c_uchar; 24]`, not
`[c_char; 24]`.  The pinned header spells each as `char[24]`, and the exact
Phase-0 compile commands in `rewrite/FILE_MAP.tsv` select
`--target=x86_64-linux-gnu` or `--target=aarch64-linux-gnu` together with
`-funsigned-char`.  `c_uchar` therefore preserves the frozen C unsigned-byte
semantics as well as the unchanged 24-byte layout.  No conversion, string
validity, allocation, reference, or ownership behavior was added.

The enum tag surfaces remain `c_int` aliases and their unqualified constants
retain their original `int` values: command `0..9`, flag masks
`0x1,0x2,0x4,0x8,0x10`, and name types `0..4`.  The frozen commands have no
`-fshort-enums`, and every enumerator fits the signed 32-bit C `int` used by
the selected targets.  The `#[repr(C)]` union and enclosing struct preserve
the source order and natural ABI: the union is 24 bytes with 4-byte alignment;
the struct fields begin at offsets `0`, `4`, `28`, and `52`, with size `56`
including trailing padding and alignment `4` on both frozen targets.

## Semantic records closed

All 16 S016178 `SYMBOLS.tsv` rows are `COMPLETE`: the only conditional is the
ordinary UAPI include guard, its macro is defined on first inclusion, and the
five declarations are unconditional for both frozen architectures.  All 10
`ABI.tsv` rows record the enum values, fixed target ABI, `-funsigned-char`
provenance, and raw `repr(C)` layouts.  All 10 `LIFETIMES.tsv` rows are
`COMPLETE`: constants have no lifetime/locking family; the aggregate is
caller/user-owned raw UAPI storage; and the union's active member is selected
by the command without Rust references, allocation, Drop, locks, RCU, or
refcounting.  There are no ABI or lifetime rows for the include guard;
those families are explicitly not applicable to a preprocessor-only guard.

No source change was needed beyond the signedness correction.  No build,
formatter, compiler, test, runtime, or benchmark command was run.

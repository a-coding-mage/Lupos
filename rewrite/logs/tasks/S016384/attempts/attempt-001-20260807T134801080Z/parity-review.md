# S016384 parity review — slot 1

Reviewed pinned `vendor/linux/include/uapi/linux/snmp.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against the current candidate
`src/include/uapi/linux/snmp.rs`, its candidate snapshot, implementation
evidence, and sealed semantic proposal.

The candidate exports all 296 C anonymous-enum enumerators under their original
UAPI names as `i32` constants with the source-order values: eight sequences of
39, 31, 8, 17, 11, 137, 34, and 19 enumerators.  It also preserves both
integer macros, `__ICMPMSG_MIB_MAX` and `__ICMP6MSG_MIB_MAX`, at `512`.  There
are no selected conditional branches beyond the C include guard, no
architecture-specific branches, and no UAPI name or value discrepancy in the
candidate source.

## Finding

- `PARITY-S016384-001` — The required evidence is internally inaccurate:
  [implementation.md](implementation.md) and the bound
  [candidate.diff](candidate.diff) state that the pinned header has “six
  anonymous C enums.”  The pinned source instead contains eight anonymous enum
  definitions, beginning at lines 19, 69, 110, 129, 155, 171, 313, and 352.
  The same eight source groups are explicitly represented by sealed proposal
  keys `SC1-6fd98c9b12dae7c7080baed1e4e15374e145e04bc7fc3951920c66f90e7ecaf5`,
  `SC1-9d69ed4e1b23cf1d6c9b166cd12174760fb9f7b37bd2889f1f198cee7608050c`,
  `SC1-96a8ed4ff49abaa5cb4a8ca6392558afb2b9403e1db4cd4eb395c3c0ec9c009a`,
  `SC1-3c38a12eb064f21bce45b0f47b57be4784d02952786aece910629d7f7d374a26`,
  `SC1-4e56c66f8049cbc9855c3fb87b4ba3fb6e8857eef4bda7d00336e62d11940eb1`,
  `SC1-0523acbade7a9e1261b5e73a5dfd348da882de1bdbef911662f758f8df1116a0`,
  `SC1-a43fba1f37b8e41ebe52bc053e512c7093254bd536812d80a8849747d95065b7`,
  and `SC1-2436a8a656384ce2131946f696c8b6b9e858100d0524e326fe9ab3d63ea42b0e`
  for AArch64 (with corresponding x86_64 records).  Correct the factual claim
  and reseal the candidate-bound closure before acceptance.  The Rust source
  itself already contains all eight sequences.

No compiler, formatter, test, or diagnostic was run.

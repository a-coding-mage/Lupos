# Parity review — S016099

Reviewed `src/include/uapi/linux/dev_energymodel.rs` against the complete
pinned `vendor/linux/include/uapi/linux/dev_energymodel.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, for the frozen `aarch64` scope.
This was a source-only review; no build, formatter, test, or runtime command
was run.

## Result

PASS — no source-level parity findings.

- Provenance identifies the exact Linux source, revision, `aarch64`, and task
  `S016099`; the SPDX expression is retained.
- Both named enum tags are represented with the C `int` ABI type, and all four
  flag enumerators retain their exact C names and values (1; 1, 2, 4).
- Every anonymous-enum attribute, command, private `__*_MAX` sentinel, and
  public `*_MAX` value is present with the original sequential value or
  `sentinel - 1` relation: perf-domain 1..5/4, perf-table 1..3/2,
  perf-state 1..7/6, and commands 1..6/5.
- `DEV_ENERGYMODEL_FAMILY_NAME` is the exact `"dev-energymodel\\0"` 16-byte
  C-char array, `DEV_ENERGYMODEL_FAMILY_VERSION` is 1, and
  `DEV_ENERGYMODEL_MCGRP_EVENT` is the exact `"event\\0"` 6-byte C-char
  array. The pinned consumers use these generated UAPI identifiers as
  generic-netlink names, versions, commands, and attributes; no name or value
  changed.
- The C header declares no structures, unions, functions, exported data, or
  conditional selected branches beyond its include guard. No unauthorized
  branding, test code, placeholder, or executable behavior was introduced.

Non-blocking evidence note: `implementation.md` and `candidate.diff` say
there are 33 enumerator/anonymous-enum constants. The header and candidate
contain 29 such constants (4 named-enum values plus 25 anonymous-enum values).
This is an evidence-summary count error only; it does not correspond to an
omitted or altered UAPI constant.

# S016105 parity review

Reviewed `vendor/linux/include/uapi/linux/dpll.h` at frozen revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/dpll.rs`.

## Result

No parity findings.

## Evidence

- The header has 14 enum tags and the candidate has exactly 14 distinct
  transparent `c_int` newtypes: `dpll_mode`, `dpll_lock_status`,
  `dpll_lock_status_error`, `dpll_clock_quality_level`, `dpll_type`,
  `dpll_pin_type`, `dpll_pin_direction`, `dpll_pin_state`,
  `dpll_pin_operstate`, `dpll_pin_capabilities`, `dpll_feature_state`,
  `dpll_a`, `dpll_a_pin`, and `dpll_cmd` (upstream lines 20--306).
- Exhaustive exported-name comparison found all 132 header enumerator/macro
  names in the candidate, including every private `__*_MAX` and public
  `*_MAX`. Values, including the zero-based feature-state enum, one-based
  attribute/command enums, flag values 1/2/4, and all derived maxima, match
  the upstream definitions.
- All ten macros are present with their exact values: numeric values use
  `c_int`; `DPLL_FAMILY_NAME` and `DPLL_MCGRP_MONITOR` retain their exact byte
  strings and terminating NULs. The header has no configuration conditional
  other than its include guard, and the Rust file introduces no configuration
  gate.
- The transparent `c_int` representations preserve the two selected Linux
  target UAPI enum integer ABI. There are no structs, functions, bitfields,
  packed layouts, linkage declarations, or architecture-specific branches in
  this header.
- SPDX is exactly `((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`; source
  path, frozen revision, `common` architecture scope, and task ID provenance
  match `vendor/linux.SHA`, the task row, and the source header.

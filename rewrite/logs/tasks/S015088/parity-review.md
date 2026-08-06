# S015088 parity review

Reviewed `vendor/linux/include/linux/sunrpc/gss_err.h` at frozen revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/sunrpc/gss_err.rs`.

## Result

One provenance finding; no behavioral, value, type-width, configuration, or
selected-symbol parity finding.

## Finding

1. **P1 — upstream copyright/permission notice omitted (low).** The candidate
   retains the 2002 Regents of the University of Michigan notice but omits the
   complete 1993 OpenVision Technologies copyright and permission/disclaimer
   notice at upstream lines 11--31. The source-tree rule requires relevant
   upstream copyright notices to be retained. Restore that notice (as a Rust
   comment) without changing the operative translation.

## Evidence

- The selected `OM_uint32` typedef is exactly `u32` on both frozen targets.
- All 47 object-like status/flag/offset/mask/alias macros are represented with
  their upstream values: 9 context flags, 3 credential-usage values, 2 status
  class values, indefinite lifetime, complete status, 6 offsets/masks, 3
  calling errors, 18 routine errors, 5 supplementary bits, and
  `GSS_S_CRED_UNAVAIL` as the `GSS_S_FAILURE` alias.
- All seven function-like macros are present as one-evaluation `const fn`
  helpers: `GSS_CALLING_ERROR`, `GSS_ROUTINE_ERROR`,
  `GSS_SUPPLEMENTARY_INFO`, `GSS_ERROR`, `GSS_CALLING_ERROR_FIELD`,
  `GSS_ROUTINE_ERROR_FIELD`, and `GSS_SUPPLEMENTARY_INFO_FIELD`. Their masks,
  shifts, operands, and `u32` results match the C macro expansions.
- The header has no configuration or architecture conditional beyond its C
  include guard; the candidate correctly has no Rust configuration gate. Its
  source path, Linux revision, `common` architecture scope, and S015088 task
  provenance match the frozen task row and `vendor/linux.SHA`.

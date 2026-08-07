# Rust review — S016384, attempt 2, slot 2

Result: FINDINGS.

## RUST-S016384-01 — eight terminal enum constants omitted

`vendor/linux/include/uapi/linux/snmp.h` has eight anonymous `enum`
declarations.  They contain 296 enumerators, not 288: each declaration ends
with a named, implicit terminal enumerator.  The current Rust candidate has
the preceding 288 enumerators plus the two literal macros, but omits all eight
terminal enumerators.  Consequently it removes public compile-time names and
their `int` values from the Rust module.

The missing C names and implicit values are:

| C source line | missing name | value |
| --- | --- | --- |
| 61 | `__IPSTATS_MIB_MAX` | 38 |
| 101 | `__ICMP_MIB_MAX` | 30 |
| 119 | `__ICMP6_MIB_MAX` | 7 |
| 147 | `__TCP_MIB_MAX` | 16 |
| 167 | `__UDP_MIB_MAX` | 10 |
| 309 | `__LINUX_MIB_MAX` | 136 |
| 348 | `__LINUX_MIB_XFRMMAX` | 33 |
| 372 | `__LINUX_MIB_TLSMAX` | 18 |

All values fit in the candidate's `i32` representation, so each must be a
module-level `pub const NAME: i32 = VALUE;`, following the same namespace and
provenance as its sibling C enumerators.  The two `512` macros are correctly
represented as `i32`; there is no overflow or macro-type finding.  The C
include guard has no additional Rust runtime representation requirement.

This finding maps to the current sealed proposal's per-architecture
`selection_expression` and `status` records for every name above (32 record
keys supplied in the semantic-closure slot-2 attestation).  The proposal itself
confirms the scope: 296 `enum_constant` records, 8 anonymous-enum declaration
records, and 2 macro records (306 selected semantic entities).  Thus the
candidate/implementation claim of 288 enumerators and 298 total semantic
records omits these eight source enumerators.

Source evidence: `vendor/linux/include/uapi/linux/snmp.h:19-61`,
`:69-101`, `:110-119`, `:129-147`, `:155-167`, `:171-309`, `:313-348`, and
`:352-372`; current candidate `src/include/uapi/linux/snmp.rs`.

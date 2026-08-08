# Parity review — S016344

Reviewed task `S016344` / `include/uapi/linux/psp.h` against the pinned Linux
source at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate,
the candidate diff, and the frozen task rows for both `x86_64` and `aarch64`.
No compiler, formatter, linker, test, or diagnostic was invoked.

## Findings

### F1 — `enum psp_version`: C enumerator names are missing from the UAPI namespace

Linux `enum psp_version` at `vendor/linux/include/uapi/linux/psp.h:13-18`
declares `PSP_VERSION_HDR0_AES_GCM_128`,
`PSP_VERSION_HDR0_AES_GCM_256`, `PSP_VERSION_HDR0_AES_GMAC_128`, and
`PSP_VERSION_HDR0_AES_GMAC_256` as unqualified enumeration constants in the
C ordinary-identifier namespace. The candidate at
`src/include/uapi/linux/psp.rs:11-16` exposes them only as variants of
`psp_version`; it provides no public constants with the four Linux names.
Consequently a UAPI consumer cannot resolve the Linux identifiers directly,
unlike every anonymous-enum constant in this header that the candidate exports
as a module-level `pub const`.

Frozen local evidence: the selected `SYMBOLS.tsv` rows 401056-401059 and
401125-401128 require these four names for `aarch64` and `x86_64`, and the
corresponding `ABI.tsv` rows 194482 and 194489 classify `enum psp_version` as
`UAPI_ENUM_OR_ENUM_CONSTANT`.

### F2 — `PSP_FAMILY_NAME`, `PSP_MCGRP_MGMT`, and `PSP_MCGRP_USE`: string-literal macro representation changes the UAPI contract

Linux `PSP_FAMILY_NAME`, `PSP_MCGRP_MGMT`, and `PSP_MCGRP_USE` at
`vendor/linux/include/uapi/linux/psp.h:10,95-96` are C string-literal macros:
each expands to a NUL-terminated character array when used as a string
literal. The candidate at `src/include/uapi/linux/psp.rs:7,70-71` instead
exports Rust `&str` values. A `&str` is a length-bearing Rust slice and its
specified contents omit the terminating NUL, so it is neither the macro's
textual substitution nor a C-string-compatible representation. This changes
the public UAPI surface and can change consumers that require the C literal's
terminator or array/pointer behavior.

Frozen local evidence: selected `SYMBOLS.tsv` rows 401051, 401053, 401054
for `aarch64` and 401120, 401122, 401123 for `x86_64` identify all three as
unconditional operative macros.

## Exhaustive coverage notes

The dual SPDX expression, task provenance, family version, all five anonymous
enum groups, their explicit starts and `MAX - 1` values, and the selected
include-guard branch were manually compared. No further omission, ordinal,
or unauthorized branding difference was found. The two findings above prevent
approval.

# S016011 parity review (slot 1)

Reviewed `vendor/linux/include/uapi/asm-generic/mman-common.h` at frozen
revision `425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/asm-generic/mman-common.rs`.

## Result

No parity findings.

## Evidence

- The upstream header has 53 UAPI object-like integer macros (lines 10--91),
  and the candidate has exactly 53 public `i32` constants with an exact,
  ordered identifier inventory match.  This covers the 7 `PROT_*` values, 10
  `MAP_*` mapping values, `MLOCK_ONFAULT`, 3 `MS_*` flags, 29 `MADV_*`
  selectors, `MAP_FILE`, and 4 `PKEY_*` values/masks.  All literals retain
  their exact integer values, including the high-bit-range-within-`int`
  values `PROT_GROWSDOWN` (`0x01000000`), `PROT_GROWSUP` (`0x02000000`), and
  `MAP_UNINITIALIZED` (`0x04000000`).
- These upstream unsuffixed decimal/hexadecimal literals have C `int` type on
  both frozen 32-bit-`int` Linux targets, and every value fits that type.  The
  candidate's explicit `i32` representation therefore preserves the UAPI
  integer category and all flag-mask values for x86_64 and AArch64.  The
  header declares no functions, storage, structs, unions, bitfields, linkage,
  calling conventions, or layout-sensitive ABI items.
- The sole computed macro, `PKEY_ACCESS_MASK` (upstream lines 91--92), remains
  the same source-level bitwise-or expression over `PKEY_DISABLE_ACCESS` and
  `PKEY_DISABLE_WRITE`; it evaluates to `0x3`.  The AArch64 UAPI parent
  explicitly undefines and replaces this generic mask, while x86 includes the
  generic header unchanged, matching the generic file's unconditional scope.
- The only source conditional is the C include guard.  There are no
  configuration-selected branches, and Rust module inclusion is the faithful
  non-textual analogue to omission of that guard.  Both frozen configurations
  select this common header; the candidate correctly records `common`.
- SPDX exactly retains `GPL-2.0 WITH Linux-syscall-note`.  Immutable source
  path, revision, architecture scope, and task ID provenance match the task
  row and `vendor/linux.SHA`.  No branding delta, tests, driver code, or
  placeholder behavior was introduced.

No build, formatting, test, linker, or runtime command was run.

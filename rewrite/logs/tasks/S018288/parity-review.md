# Parity review: S018288

Reviewed task `S018288` independently against pinned
`vendor/linux/security/selinux/include/policycap.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen `x86_64` scope.

## Result

No parity findings.

## Evidence checked

- The source header's complete anonymous enumeration is represented by the
  same fifteen public `i32` constants in declaration order, with values 0
  through 14, followed by `__POLICYDB_CAP_MAX` at 15.  Each C enumerator is an
  `int` constant expression here; the Rust constants retain the values and
  signed 32-bit type used by the header's expressions.
- The source's derived macro `POLICYDB_CAP_MAX` is preserved as
  `__POLICYDB_CAP_MAX - 1`, therefore evaluates to 14 rather than conflating
  the final valid index with the array bound.  Pinned consumer
  `security/selinux/selinuxfs.c:1745` iterates inclusively to this value, while
  `security/selinux/include/security.h:99` uses the separate maximum as the
  `policycap` array bound.
- The external declaration `const char *const selinux_policycap_names[__POLICYDB_CAP_MAX]`
  is represented as an immutable `extern "C"` static named
  `selinux_policycap_names` with element type `*const c_char` and a derived
  length of 15.  This preserves the symbol name, C linkage, fixed array shape,
  immutable pointer elements, and read-only character pointees.  Pinned
  `include/policycap_names.h:10-26` supplies exactly 15 corresponding entries;
  `ss/services.c:2183` and `ima.c:30` consume the bound as an array length.
- No conditional configuration branches, additional types, functions,
  storage definitions, layouts, branding changes, or executable behavior
  occur in the pinned header.  The Rust provenance identifies the exact source,
  revision, architecture, and task, and its SPDX identifier conforms to the
  project-required Rust provenance form.

The frozen Phase 0 identity records the same Linux revision and the queue
verification returned fingerprint
`af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`.

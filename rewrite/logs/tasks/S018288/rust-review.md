# Rust review — S018288

Reviewed independently as slot 2 against pinned
`vendor/linux/security/selinux/include/policycap.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` and the frozen x86_64 task context.

## Finding R1 — foreign array element type does not match the frozen C `char` type

**Severity: must fix.**

`src/security/selinux/include/policycap.rs:30` imports the C declaration as
`[*const core::ffi::c_char; __POLICYDB_CAP_MAX as usize]`.  The frozen Kbuild
compile command recorded for this header's consumer in `rewrite/FILE_MAP.tsv`
contains `-funsigned-char`; therefore, for this selected configuration,
`const char *const` in the upstream declaration is an array of pointers to
**unsigned** character objects.  On `x86_64-unknown-linux-gnu`, Rust's
target-default `core::ffi::c_char` is signed, and it does not incorporate this
per-translation-unit Kbuild option.

Upstream evidence: `policycap.h:27` declares
`extern const char *const selinux_policycap_names[__POLICYDB_CAP_MAX];`; the
definition in `security/selinux/include/policycap_names.h:10` has that same
type.  The Rust foreign declaration must model its pointee as an unsigned byte
type for this frozen C configuration (while retaining the immutable imported
array and its exact `__POLICYDB_CAP_MAX` extent), rather than relying on the
Rust target's default `c_char` signedness.

## Checked without findings

- The anonymous C enum has no tag/type name to expose; its fifteen enumerators
  are representable `int` constants, and the candidate preserves their order,
  `0..=14` values, and `i32` type.
- `__POLICYDB_CAP_MAX` is correctly `15`; the macro result
  `POLICYDB_CAP_MAX` is correctly `14` with `int`/`i32` arithmetic.
- The imported array uses the exact symbol spelling, fixed extent, pointer
  representation, and an immutable foreign static, which otherwise matches
  C's `const char *const` declaration.  No owned reference, layout-bearing
  Rust type, mutable static, or additional linkage name is introduced.

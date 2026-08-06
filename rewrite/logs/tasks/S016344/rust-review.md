# Rust review — S016344 (slot 2)

Reviewed the complete pinned `include/uapi/linux/psp.h` against
`src/include/uapi/linux/psp.rs` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the common x86_64/aarch64
scope.  This was a source-only review; no compiler, formatter, test, or
runtime command was run.

## Result

Accepted: no Rust-semantics finding.

## Checks

- `psp_version` is an alias of `core::ffi::c_int`, preserving the frozen
  signed C `int` representation of the named enum tag without introducing a
  Rust enum invalid-value restriction or a wrapper conversion at C integer
  use sites.  Its four enumerators retain values 0 through 3.
- All 55 C enumerators across the six anonymous enums and the named
  `psp_version` enum are present as public `c_int` constant expressions (the
  named-enum alias is itself `c_int`).  Explicit initializers, implicit
  increments, private `__*_MAX` sentinels, and public `*_MAX = __*_MAX - 1`
  values exactly match the pinned source.  No narrowing, wrapping, panic,
  debug/release variation, or unsigned substitution was introduced.
- `PSP_FAMILY_VERSION` remains a `c_int` value of 1.  The three
  string-literal macros are immutable NUL-terminated `[c_char; N]` statics
  with their exact source byte sequences and lengths: `psp\0` (4), `mgmt\0`
  (5), and `use\0` (4).  They retain static backing storage and a translated
  caller can express C ordinary-expression array-to-pointer decay explicitly
  with `.as_ptr()`; no Rust slice/reference, UTF-8 assumption, ownership, or
  mutable alias is exposed.
- The source has no selected architecture, Kconfig, or other conditional API
  beyond its include guard, and the candidate introduces no `cfg` divergence.
  It has no struct/union layout, extern declaration, unsafe block, allocation,
  synchronization, `Drop`, panic path, or project-authored Rust test.
- SPDX and immutable provenance identify the exact source path, revision,
  common architecture scope, and task ID.  UAPI identifiers are unchanged.

No source, manifest, index, queue, build, formatting, or test file was
modified by this reviewer.

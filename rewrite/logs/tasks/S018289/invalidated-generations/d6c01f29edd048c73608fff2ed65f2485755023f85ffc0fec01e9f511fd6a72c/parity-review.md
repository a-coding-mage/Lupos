# Parity review — S018289

Reviewed `security/selinux/include/policycap_names.h` from pinned Linux
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/security/selinux/include/policycap_names.rs`, with dependency S018288,
the frozen x86_64 scope/symbol/ABI/lifetime records, and the direct selected
consumer `security/selinux/ss/services.c`.

## Result

No parity findings.

## Verified

- The exported definition is `selinux_policycap_names`, has external linkage
  through `#[unsafe(no_mangle)]`, and is an immutable static object. Its
  transparent pointer-element wrapper has the same one-pointer object
  representation required by C `const char *const` array elements; immutable
  static storage preserves the array-pointer const qualification and the
  `*const c_uchar` pointee representation preserves byte-addressed character
  pointers.
- Its extent is exactly `__POLICYDB_CAP_MAX as usize`, supplied by completed
  dependency S018288. The dependency's enum gives `__POLICYDB_CAP_MAX = 15`;
  the definition contains exactly 15 elements. This agrees with both
  `ARRAY_SIZE(selinux_policycap_names)` loops and index checks in the selected
  `services.c` consumer.
- Each source string is represented once, in upstream order, with the exact
  ASCII spelling and one trailing NUL byte. The byte-string backing storage is
  static, so every stored pointer remains valid for the global's full lifetime.
- Candidate provenance identifies the exact Linux source, pinned revision,
  x86_64 scope, and task. The selected scope row is `RUST_TRANSLATE` for this
  exact destination and has only S018288 as a dependency. No unallowlisted
  branding, placeholder, or Rust test configuration is present.

No compiler, formatter, linker, test, or runtime command was used.

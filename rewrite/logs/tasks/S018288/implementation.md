# Implementation: S018288

- Task: `security/selinux/include/policycap.h` → `src/security/selinux/include/policycap.rs`
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architecture: `x86_64`
- Implementer model/effort: `gpt-5.6-terra` / `medium` (fallback because Luna was unavailable)

Read the complete pinned header and its x86_64 selected consumer context: the
policy capability name definition, `security.h` state and inline uses,
`ss/services.c`, and `selinuxfs.c`. The C anonymous enum has the default
`int` representation and explicitly sequential values 0 through 15. It is
represented by `i32` constants with the original names and values. The macro
`POLICYDB_CAP_MAX` remains the derived `__POLICYDB_CAP_MAX - 1` value. The
external C declaration remains an `unsafe extern "C"` immutable static array
of immutable C character pointers with the exact length 15.

No build, formatter, compiler, test, or runtime command was run.

# Resolution — S013482

Applied source-level resolution against pinned
`vendor/linux/include/linux/audit_arch.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`. No compiler, formatter, linker,
test, diagnostic, or historical Lupos source was used.

## Parity review findings

1. **P1 — resolved.** Linux lines 27–31 declare five external incomplete
   `unsigned int` arrays. The corresponding pinned aarch64 definitions in
   `lib/compat_audit.c:7–30` are variable-length arrays terminated with
   `~0U`, and `lib/audit.c:75–79` passes their decayed addresses to
   `audit_register_class`. Each Rust binding is now `static mut [u32; 0]`:
   the external symbol remains address-only opaque array storage, with no
   scalar element representation or known Rust extent. A future caller must
   form a raw `*mut u32` from the symbol address to mirror C array-to-pointer
   decay; it must not create a Rust array or scalar reference.
2. **P2 — resolved.** The source now retains the pinned
   `GPL-2.0-or-later` SPDX identifier and the complete 2021 Red Hat/Richard
   Guy Briggs copyright and attribution notice.

## Rust review findings

1. **R1 — resolved.** The five bindings are zero-length external arrays, not
   scalar statics. Their non-const C definitions remain represented by
   `static mut`; the declaration itself supplies no safe Rust reference or
   element access.
2. **R2 — resolved.** The exact upstream SPDX/copyright/author text is
   present after the immutable task provenance.
3. **Enum validity and copy behavior — resolved.** The pinned header uses
   `enum auditsc_class_t` only to name the integral classification constants
   0 through 7; selected definitions return those constants through `int`
   (`lib/compat_audit.c:32–56`, `lib/audit.c:40–70`,
   `arch/x86/ia32/audit.c:31–48`, and
   `arch/x86/kernel/audit_64.c:65–83`). It is neither an FFI parameter nor an
   exported object in this header. A Rust `#[repr(C)]` enum would restrict
   values to its eight discriminants and would not model C's freely copied
   integer classification values. The final `i32` alias preserves the `int`
   classification representation and every `i32` value, while its constants
   preserve the source order and values exactly; alias values are Copy by
   Rust's scalar semantics, matching C integer copy behavior.

## Pending semantic records closed

- `enum auditsc_class_t`: integer classification constants, represented as
  `i32` because all selected uses return them through C `int`; all values
  representable by that classification result remain valid and copyable.
- `compat_{write,read,dir,chattr,signal}_class`: externally owned mutable,
  incomplete `unsigned int` arrays. Their extent and contents are defined by
  the selected implementation; this header contract exposes only the raw
  address of element zero for compatibility-class registration.
- `audit_classify_compat_syscall`: unconditional external C function with
  `int`/`unsigned int` parameters and `int` result, represented as
  `extern "C" fn(i32, u32) -> i32`.

All selected declarations are unconditional for both frozen architectures. No
branding delta applies.

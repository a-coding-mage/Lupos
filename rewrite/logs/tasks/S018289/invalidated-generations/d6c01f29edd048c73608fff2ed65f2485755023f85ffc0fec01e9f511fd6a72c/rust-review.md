# Rust semantics review — S018289

## Result

ACCEPT.  Manual source review found no Rust ownership, layout, linkage, or
literal-lifetime defect in `src/security/selinux/include/policycap_names.rs`.

## Evidence reviewed

- Pinned source: `vendor/linux/security/selinux/include/policycap_names.h`
  (lines 1–29), at revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Dependency S018288: pinned `policycap.h` and its completed translation
  `src/security/selinux/include/policycap.rs`.  It fixes
  `__POLICYDB_CAP_MAX` at 15 and declares the C-facing array as 15 pointer
  slots.
- Selected callers in the pinned source (`security/selinux/ima.c`,
  `security/selinux/selinuxfs.c`, and `security/selinux/ss/services.c`) only
  index the array and pass the resulting NUL-terminated `const char *` values
  to string/printing operations.

## Rust-semantics findings

1. `selinux_policycap_name` is a one-field `#[repr(transparent)]` wrapper
   around `*const c_uchar`.  Consequently each array element has the pointer
   representation, alignment, and size required by C `const char *`; the
   outer immutable Rust `static` supplies the outer `const` in C
   `const char *const`.  The S018288 external declaration's raw-pointer array
   has the same FFI storage layout.
2. The explicit `unsafe impl Sync` is appropriately narrow and justified:
   this exported immutable array contains only pointers to immutable static
   byte-string literals.  It does not add `Send`, create references, or allow
   safe mutation of the global.  Reading the public tuple field only copies a
   raw pointer; dereferencing it remains unsafe.
3. Each `b"...\\0".as_ptr()` denotes the corresponding immutable,
   NUL-terminated byte literal for the lifetime of the program.  All 15 source
   strings appear once, in original order, and the bound uses the dependency's
   value 15, so there is neither an omitted element nor out-of-bounds
   initializer evaluation.
4. `#[unsafe(no_mangle)] pub static` preserves the external data-symbol name
   `selinux_policycap_names`.  It is deliberately not `static mut`; neither
   the pointer slots nor the pointee bytes are writable through safe Rust.
   No additional unsafe block or unchecked reference/pointer operation was
   introduced.
5. The immutable provenance header identifies the exact Linux source,
   revision, architecture, and task.  The source has no additional copyright
   notice to retain; the candidate carries the required GPL-2.0-only SPDX
   provenance header.

No compilation, formatter, test, or diagnostic tool was used.

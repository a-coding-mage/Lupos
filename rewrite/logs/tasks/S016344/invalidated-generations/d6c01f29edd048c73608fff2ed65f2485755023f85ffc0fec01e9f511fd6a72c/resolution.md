# S016344 applier resolution

Pinned source reopened in full: `vendor/linux/include/uapi/linux/psp.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the exact revision recorded in
`vendor/linux.SHA`.  The source is an unconditional common-architecture YNL
generic-netlink UAPI header; its only conditional is the C multiple-inclusion
guard.

## Review dispositions

1. **Parity review — accepted in part.**  Its complete 55-enumerator and
   numeric-value reconciliation is correct.  During independent source-context
   recheck, however, the original public Rust string-array items were adjusted:
   a C string-literal macro is a `const char *` expression after ordinary array
   decay, not an array object requiring a caller-side conversion.
2. **Rust review — accepted in part.**  The `c_int` mapping of the named
   `psp_version` enum and all anonymous-enum constants avoids a Rust-enum
   invalid-value restriction and preserves C integer values.  The same
   string-expression correction was required for the three C string macros.

## Applied reconciliation

- `PSP_FAMILY_NAME`, `PSP_MCGRP_MGMT`, and `PSP_MCGRP_USE` now expose immutable
  `*const c_char` values backed by private static NUL-terminated arrays.  This
  retains the exact byte sequences `psp\\0`, `mgmt\\0`, and `use\\0`, their
  static lifetime, and the pointer expression produced by C array-to-pointer
  decay.  The pinned consumer `net/psp/psp-nl-gen.c:161-172` uses
  `PSP_FAMILY_NAME` directly as the generic-netlink family `.name`, confirming
  why the macro-equivalent pointer form matters.  `PSP_FAMILY_VERSION` remains
  the C `int` value `1`.
- Direct source-to-candidate recheck found all 55 enumerators, in their source
  order and with their exact signed C `int` values: the four `psp_version`
  ordinals `0..3`; association-device-info `1,2,3,2`; device `1..8,7`;
  association `1..6,5`; keys `1..3,2`; statistics `1..12,11`; and commands
  `1..13,12`.  Each private `__PSP_*_MAX` sentinel and public
  `PSP_*_MAX = sentinel - 1` expression is retained.
- The SPDX expression and immutable provenance exactly identify the upstream
  source, pinned revision, `common` architecture scope, and task ID.  There is
  no branding change, structure layout, function, linkage, configuration
  branch, allocation, ownership transfer, locking, RCU/refcount, callback,
  cleanup, unsafe boundary, placeholder, or Rust test in this file.

The outstanding task-local semantic records for both frozen architectures are
therefore closed as follows: `psp_version` and every anonymous enumerator are
by-value C-`int` definitions; the three string macros have immutable
static-duration C-character storage and pointer-expression semantics; all
ownership/lifetime, locking/RCU/refcount, and layout/calling-convention
categories not stated above are `NOT_APPLICABLE`.  No source outside the leased
destination and this task resolution was edited.  No compiler, formatter,
build, linker, test, emulator, debugger, benchmark, or runtime command was
run.

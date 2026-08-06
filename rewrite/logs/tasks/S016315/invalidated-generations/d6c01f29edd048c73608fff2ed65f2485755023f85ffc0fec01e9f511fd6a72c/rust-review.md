# Rust semantics review — S016315

Reviewed `vendor/linux/include/uapi/linux/nfsacl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/nfsacl.rs`.  This was a manual, source-only review.

## Findings

1. **Must resolve — fixed `i32` annotations erase the source macros’
   context-dependent integer conversions.**  Every source replacement list is
   a small `int`-typed integer constant on both frozen targets, but macro
   expansion is then subject to the type required by its use.  In particular,
   the source uses the ACL flag macros with `__u32` protocol masks
   (`fs/nfsd/xdr3.h:103-106` and `fs/nfsd/nfs3acl.c:43`), where C’s usual
   arithmetic conversions convert the `int` replacement to `unsigned int`.
   The candidate exports the same names exclusively as `i32`
   (`src/include/uapi/linux/nfsacl.rs:23-30`), which cannot participate in the
   equivalent Rust `u32` bit operations without a separate conversion at every
   such call site.  The same issue applies to the program/procedure constants
   when their destination is unsigned.  This is an observable public-surface
   and integer-semantics mismatch; resolve it with an upstream-evidence-based
   representation/use mapping that preserves each selected signed and unsigned
   context rather than treating every macro as an `i32` constant.

2. **Must resolve — upstream copyright notice was not retained.**  The source
   has `(C) 2003 Andreas Gruenbacher <agruen@suse.de>` at
   `include/uapi/linux/nfsacl.h:5`; the candidate retains the SPDX expression
   but omits that relevant upstream copyright notice.  Add the immutable notice
   while retaining the required provenance header.

## Checked without findings

- The candidate contains all 15 value-bearing source macros with their exact
  numeric values; no executable behavior, layout, FFI object, or exported C
  linkage exists in this header.
- SPDX expression, Linux source path, frozen revision, architecture membership,
  and task provenance match the pinned source and queue row.
- No Rust test configuration, test item, placeholder, panic stub, or mutable
  completion claim is present.
- The C include guard has no Rust module-level ABI equivalent.

## Verdict

Changes required before source acceptance: resolve both findings above.

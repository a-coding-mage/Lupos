# Parity Review — S016567 (slot 1)

Reviewed `src/include/xen/interface/features.rs` against the complete pinned
`vendor/linux/include/xen/interface/features.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen AArch64 scope, and the
S016567 symbol inventory.  This was a source-only review; no build or test was
run.

## Result: changes required

1. **`u32` changes every active C macro's integer type and use-site semantics.**
   The upstream replacement lists decimal integer literals (`0` through `17`
   and `1`), each of which has C `int` type on the frozen kernel target.  The
   candidate instead gives all 18 active macro replacements `u32` type.  This
   is observable at typed Rust use sites: upstream passes feature identifiers
   to `static inline int xen_feature(int flag)` in `include/xen/features.h`,
   whereas the candidate constants require either a changed callee signature or
   a cast.  Preserve the C `int` representation (the frozen targets use a
   32-bit `int`, so `i32`) or document and preserve each equivalent use-site
   conversion without changing the exposed interface.  Evidence: upstream
   lines 17, 23, 31, 34, 40, 43, 46, 52, 55, 58, 61, 64, 72, 75, 84, 97, 98,
   and 100; `vendor/linux/include/xen/features.h:19`.

2. **The required upstream copyright notice is missing.**
   The candidate retains the MIT SPDX identifier but omits `Copyright (c) 2006,
   Keir Fraser <keir@xensource.com>` from upstream line 7.  The rewrite policy
   requires retention of relevant upstream copyright notices.  Restore it in
   the Rust header.

3. **Relevant feature-contract commentary is materially incomplete.**
   The candidate compresses or drops the upstream conditions that distinguish
   these public feature indices: the feature-12 definition is intentionally
   disabled because `XENFEAT_grant_map_identity` is deprecated; the RSDP
   relaxation applies only when `XEN_ELFNOTE_GUEST_OS` contains `"linux"`; and
   the direct-map feature pair has explicit fallback assumptions for older Xen
   on x86 and ARM.  These are the semantics by which callers interpret the
   flags, not decorative commentary.  Carry these conditions forward in the
   Rust documentation adjacent to the corresponding constants.  Evidence:
   upstream lines 66–70, 77–95.

## Verified parity points

- Provenance is exact: source path, task ID, AArch64 membership, and Linux SHA
  all match the frozen queue and `vendor/linux.SHA`.
- Every active feature macro is present with the original identifier and value:
  0–11, 13–17, and `XENFEAT_NR_SUBMAPS = 1`.  No active macro is omitted or
  added.
- `XENFEAT_grant_map_identity` is correctly absent: its apparent `#define` is
  inside upstream's block comment, so it is not a C preprocessor macro.
- The source include guard has no Rust module-level analogue and adds no
  runtime, ABI, storage, function, or configuration behavior.
- Candidate comments do not contradict the source, but finding 3 requires the
  omitted operational qualifications to be retained in the translated header.

## Scope / ABI conclusion

This header exports only preprocessor integer constants.  It declares no
layout, storage, linkage, callable symbol, or architecture conditional.  The
only ABI-relevant correction is the typed-constant issue above; no separate
FFI declaration is required by this file.
